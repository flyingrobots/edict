//! Immutable artifact-schema authority for external Edict providers.
//!
//! Registry construction consumes only a validated provider manifest and
//! explicit in-memory schema bytes. It verifies each raw schema digest and
//! compiles every self-contained CDDL document before an external component can
//! run. Validation performs no discovery, file access, network access, clock,
//! randomness, environment inspection, or mutable-global lookup.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use cddl_cat::cbor::validate_cbor;
use cddl_cat::context::BasicContext;
use cddl_cat::flatten::flatten_from_str;
use cddl_cat::ivt::{Control, Node, RuleDef};
use edict_syntax::{
    encode_canonical_cbor, CanonicalValue, ProviderArtifactSchemaValidationErrorKind,
    ProviderArtifactSchemaValidator, ProviderArtifactSource, ProviderSchemaBinding,
    ProviderSchemaFormat, ResourceRef, ValidatedTargetProviderManifest,
};
use sha2::{Digest, Sha256};

/// Explicit bytes alleged to implement one manifest schema role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProviderSchemaArtifact {
    pub role: String,
    pub bytes: Arc<[u8]>,
}

/// Stable construction failure categories for the immutable registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSchemaRegistryFailureKind {
    SchemaArtifactMissing,
    SchemaArtifactAmbiguous,
    SchemaArtifactDigestMismatch,
    SchemaCompileFailed,
    SchemaRootRuleMissing,
    RequiredDomainMissing,
}

/// One deterministic registry-construction failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSchemaRegistryFailure {
    kind: ProviderSchemaRegistryFailureKind,
    domain: Option<String>,
    schema_role: Option<String>,
}

impl ProviderSchemaRegistryFailure {
    #[must_use]
    pub const fn kind(&self) -> ProviderSchemaRegistryFailureKind {
        self.kind
    }

    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    #[must_use]
    pub fn schema_role(&self) -> Option<&str> {
        self.schema_role.as_deref()
    }

    fn for_role(kind: ProviderSchemaRegistryFailureKind, schema_role: &str) -> Self {
        Self {
            kind,
            domain: None,
            schema_role: Some(schema_role.to_owned()),
        }
    }

    fn for_binding(
        kind: ProviderSchemaRegistryFailureKind,
        domain: &str,
        schema_role: &str,
    ) -> Self {
        Self {
            kind,
            domain: Some(domain.to_owned()),
            schema_role: Some(schema_role.to_owned()),
        }
    }

    fn for_domain(kind: ProviderSchemaRegistryFailureKind, domain: &str) -> Self {
        Self {
            kind,
            domain: Some(domain.to_owned()),
            schema_role: None,
        }
    }
}

impl fmt::Display for ProviderSchemaRegistryFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.kind)?;
        if let Some(domain) = &self.domain {
            write!(formatter, ": domain {domain}")?;
        }
        if let Some(role) = &self.schema_role {
            write!(formatter, ": schema role {role}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderSchemaRegistryFailure {}

/// Immutable evidence for one compiled domain binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSchemaRegistryBinding {
    pub domain: String,
    pub schema_role: String,
    pub schema: ResourceRef,
    pub source: ProviderArtifactSource,
    pub format: ProviderSchemaFormat,
    pub root_rule: String,
}

struct CompiledSchema {
    context: Arc<BasicContext>,
    root_rule: String,
}

/// Complete immutable mapping from artifact domains to compiled schemas.
pub struct ProviderArtifactSchemaRegistry {
    manifest: edict_syntax::TargetProviderManifest,
    schemas: BTreeMap<String, CompiledSchema>,
    bindings: Vec<ProviderSchemaRegistryBinding>,
}

impl fmt::Debug for ProviderArtifactSchemaRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderArtifactSchemaRegistry")
            .field("bindings", &self.bindings)
            .field("provider", &self.manifest.provider)
            .finish_non_exhaustive()
    }
}

impl ProviderArtifactSchemaRegistry {
    /// Construct and compile a complete registry from explicit schema bytes.
    ///
    /// `required_domains` is the host-authored closure needed by the prepared
    /// invocation. Every manifest binding is still compiled so no latent,
    /// digest-locked schema failure can be deferred until after guest execution.
    ///
    /// # Errors
    ///
    /// Returns a stable failure for missing or ambiguous bytes, digest
    /// disagreement, invalid or incomplete CDDL, a missing root, or an unbound
    /// required domain.
    pub fn from_manifest<'a>(
        validated: &ValidatedTargetProviderManifest<'_>,
        resolved: impl IntoIterator<Item = ResolvedProviderSchemaArtifact>,
        required_domains: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, ProviderSchemaRegistryFailure> {
        let mut resolved_by_role = BTreeMap::new();
        for artifact in resolved {
            let role = artifact.role.clone();
            if resolved_by_role.insert(role.clone(), artifact).is_some() {
                return Err(ProviderSchemaRegistryFailure::for_role(
                    ProviderSchemaRegistryFailureKind::SchemaArtifactAmbiguous,
                    &role,
                ));
            }
        }

        let manifest = validated.manifest();
        let artifacts_by_role: BTreeMap<&str, _> = manifest
            .artifacts
            .iter()
            .map(|artifact| (artifact.role.as_str(), artifact))
            .collect();
        let mut compiled_by_role: BTreeMap<String, Arc<BasicContext>> = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        let mut bindings = Vec::with_capacity(manifest.schema_bindings.len());

        for binding in &manifest.schema_bindings {
            let Some(artifact) = artifacts_by_role.get(binding.schema_role.as_str()) else {
                return Err(ProviderSchemaRegistryFailure::for_binding(
                    ProviderSchemaRegistryFailureKind::SchemaCompileFailed,
                    &binding.domain,
                    &binding.schema_role,
                ));
            };
            let resolved = resolved_by_role.get(&binding.schema_role).ok_or_else(|| {
                ProviderSchemaRegistryFailure::for_binding(
                    ProviderSchemaRegistryFailureKind::SchemaArtifactMissing,
                    &binding.domain,
                    &binding.schema_role,
                )
            })?;
            let resolved_digest = digest_review_string(&resolved.bytes);
            if Some(resolved_digest.as_str()) != artifact.resource.digest.as_deref() {
                return Err(ProviderSchemaRegistryFailure::for_binding(
                    ProviderSchemaRegistryFailureKind::SchemaArtifactDigestMismatch,
                    &binding.domain,
                    &binding.schema_role,
                ));
            }

            let context = compiled_context_for(binding, resolved, &mut compiled_by_role)?;
            if !context.rules.contains_key(&binding.root_rule) {
                return Err(ProviderSchemaRegistryFailure::for_binding(
                    ProviderSchemaRegistryFailureKind::SchemaRootRuleMissing,
                    &binding.domain,
                    &binding.schema_role,
                ));
            }

            schemas.insert(
                binding.domain.clone(),
                CompiledSchema {
                    context,
                    root_rule: binding.root_rule.clone(),
                },
            );
            bindings.push(ProviderSchemaRegistryBinding {
                domain: binding.domain.clone(),
                schema_role: binding.schema_role.clone(),
                schema: artifact.resource.clone(),
                source: artifact.source.clone(),
                format: binding.format,
                root_rule: binding.root_rule.clone(),
            });
        }

        let required: BTreeSet<&str> = required_domains.into_iter().collect();
        for domain in required {
            if !schemas.contains_key(domain) {
                return Err(ProviderSchemaRegistryFailure::for_domain(
                    ProviderSchemaRegistryFailureKind::RequiredDomainMissing,
                    domain,
                ));
            }
        }

        Ok(Self {
            manifest: manifest.clone(),
            schemas,
            bindings,
        })
    }

    /// Return the exact validated manifest that authorized this registry.
    #[must_use]
    pub const fn manifest(&self) -> &edict_syntax::TargetProviderManifest {
        &self.manifest
    }

    /// Return the sorted immutable registry receipt.
    #[must_use]
    pub fn bindings(&self) -> &[ProviderSchemaRegistryBinding] {
        &self.bindings
    }
}

impl ProviderArtifactSchemaValidator for ProviderArtifactSchemaRegistry {
    fn supports_domain(&self, domain: &str) -> bool {
        self.schemas.contains_key(domain)
    }

    fn validate_canonical_value(
        &self,
        domain: &str,
        value: &CanonicalValue,
    ) -> Result<(), ProviderArtifactSchemaValidationErrorKind> {
        let schema = self
            .schemas
            .get(domain)
            .ok_or(ProviderArtifactSchemaValidationErrorKind::UnsupportedDomain)?;
        let bytes = encode_canonical_cbor(value)
            .map_err(|_| ProviderArtifactSchemaValidationErrorKind::SchemaMismatch)?;
        let cbor_value: ciborium::Value = ciborium::from_reader(bytes.as_slice())
            .map_err(|_| ProviderArtifactSchemaValidationErrorKind::SchemaMismatch)?;
        let rule = schema
            .context
            .rules
            .get(&schema.root_rule)
            .ok_or(ProviderArtifactSchemaValidationErrorKind::SchemaMismatch)?;
        validate_cbor(rule, &cbor_value, schema.context.as_ref())
            .map_err(|_| ProviderArtifactSchemaValidationErrorKind::SchemaMismatch)
    }
}

fn digest_review_string(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn compiled_context_for(
    binding: &ProviderSchemaBinding,
    resolved: &ResolvedProviderSchemaArtifact,
    compiled_by_role: &mut BTreeMap<String, Arc<BasicContext>>,
) -> Result<Arc<BasicContext>, ProviderSchemaRegistryFailure> {
    if let Some(context) = compiled_by_role.get(&binding.schema_role) {
        return Ok(Arc::clone(context));
    }
    let failure = || {
        ProviderSchemaRegistryFailure::for_binding(
            ProviderSchemaRegistryFailureKind::SchemaCompileFailed,
            &binding.domain,
            &binding.schema_role,
        )
    };
    let source = std::str::from_utf8(&resolved.bytes).map_err(|_| failure())?;
    let rules = flatten_from_str(source).map_err(|_| failure())?;
    if !rules_are_self_contained(&rules) {
        return Err(failure());
    }
    let context = Arc::new(BasicContext::new(rules));
    compiled_by_role.insert(binding.schema_role.clone(), Arc::clone(&context));
    Ok(context)
}

fn rules_are_self_contained(rules: &BTreeMap<String, RuleDef>) -> bool {
    rules
        .iter()
        .all(|(_, rule)| node_is_self_contained(&rule.node, rules, &rule.generic_parms))
}

fn node_is_self_contained(
    node: &Node,
    rules: &BTreeMap<String, RuleDef>,
    generic_parameters: &[String],
) -> bool {
    match node {
        Node::Literal(_) | Node::PreludeType(_) => true,
        Node::Rule(rule) | Node::Unwrap(rule) | Node::Choiceify(rule) => {
            (rules.contains_key(&rule.name) || generic_parameters.contains(&rule.name))
                && rule
                    .generic_args
                    .iter()
                    .all(|argument| node_is_self_contained(argument, rules, generic_parameters))
        }
        Node::Choice(choice) => choice
            .options
            .iter()
            .all(|option| node_is_self_contained(option, rules, generic_parameters)),
        Node::Map(map) => map
            .members
            .iter()
            .all(|member| node_is_self_contained(member, rules, generic_parameters)),
        Node::Array(array) | Node::ChoiceifyInline(array) => array
            .members
            .iter()
            .all(|member| node_is_self_contained(member, rules, generic_parameters)),
        Node::Group(group) => group
            .members
            .iter()
            .all(|member| node_is_self_contained(member, rules, generic_parameters)),
        Node::KeyValue(pair) => {
            node_is_self_contained(&pair.key, rules, generic_parameters)
                && node_is_self_contained(&pair.value, rules, generic_parameters)
        }
        Node::Occur(occur) => node_is_self_contained(&occur.node, rules, generic_parameters),
        Node::Range(range) => {
            node_is_self_contained(&range.start, rules, generic_parameters)
                && node_is_self_contained(&range.end, rules, generic_parameters)
        }
        Node::Control(control) => control_is_self_contained(control, rules, generic_parameters),
    }
}

fn control_is_self_contained(
    control: &Control,
    rules: &BTreeMap<String, RuleDef>,
    generic_parameters: &[String],
) -> bool {
    let both = |left: &Node, right: &Node| {
        node_is_self_contained(left, rules, generic_parameters)
            && node_is_self_contained(right, rules, generic_parameters)
    };
    match control {
        Control::Size(value) => both(&value.target, &value.size),
        Control::Lt(value) => both(&value.target, &value.lt),
        Control::Le(value) => both(&value.target, &value.le),
        Control::Gt(value) => both(&value.target, &value.gt),
        Control::Ge(value) => both(&value.target, &value.ge),
        Control::Regexp(_) => true,
        _ => false,
    }
}
