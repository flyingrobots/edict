//! Lowerability checks for typed v1 requirement artifacts.
//!
//! These tests assert public behavior: classification and stable failure kinds.
//! They do not inspect diagnostic prose, serialized bytes, or documentation
//! layout.

use edict_syntax::{
    check_lowerability, AtomicityRequirement, DirectAdapterSupport, GuardKind,
    LowerabilityFailureKind, LowerabilityStatus, LoweringRequirements, NativeEffectSupport,
    ResourceRef, SemanticEffectRequirement, TargetProfileFacts, WriteClass,
};

fn read_requirements() -> LoweringRequirements {
    LoweringRequirements {
        operation_profile: "kv.transactional@1.readOnly".to_owned(),
        semantic_effects: vec![SemanticEffectRequirement {
            coordinate: "hello.optics@1.readGreeting".to_owned(),
            write_class: WriteClass::Read,
            guard_kinds: vec![GuardKind::PrecommitAtomic],
            obstruction_coordinates: vec!["hello.optics@1.GreetingMissing".to_owned()],
            footprint_obligations: vec!["hello.optics@1.readGreeting.footprint".to_owned()],
            cost_obligations: vec!["hello.optics@1.readGreeting.cost".to_owned()],
        }],
        required_write_classes: vec![WriteClass::Read],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: AtomicityRequirement::Atomic,
        postcondition_support: true,
        obstruction_coordinates: vec!["hello.optics@1.GreetingMissing".to_owned()],
        footprint_obligations: vec!["hello.optics@1.readGreeting.footprint".to_owned()],
        cost_obligations: vec!["hello.optics@1.readGreeting.cost".to_owned()],
        optic_contract: "read-only-point".to_owned(),
    }
}

fn profile_facts() -> TargetProfileFacts {
    TargetProfileFacts {
        coordinate: "kv.transactional@1".to_owned(),
        operation_profiles: vec!["kv.transactional@1.readOnly".to_owned()],
        native_effects: vec![NativeEffectSupport {
            coordinate: "hello.optics@1.readGreeting".to_owned(),
            target_intrinsic: "kv.transactional@1.get".to_owned(),
            write_class: WriteClass::Read,
        }],
        direct_adapters: Vec::new(),
        write_classes: vec![WriteClass::Read],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: vec![AtomicityRequirement::Atomic],
        postcondition_support: true,
        obstruction_coordinates: vec!["hello.optics@1.GreetingMissing".to_owned()],
        footprint_obligations: vec!["hello.optics@1.readGreeting.footprint".to_owned()],
        cost_obligations: vec!["hello.optics@1.readGreeting.cost".to_owned()],
        optic_contracts: vec!["read-only-point".to_owned()],
    }
}

fn direct_adapter() -> DirectAdapterSupport {
    DirectAdapterSupport {
        semantic_effect: "hello.optics@1.readGreeting".to_owned(),
        target_intrinsic: "kv.transactional@1.get".to_owned(),
        adapter: ResourceRef {
            coordinate: "hello.optics@1.kv.transactional.adapter/v1".to_owned(),
            digest: Some(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_owned(),
            ),
        },
        write_class: WriteClass::Read,
        emits_semantic_effects: Vec::new(),
    }
}

#[test]
fn native_target_facts_satisfy_lowering_requirements() {
    let report = check_lowerability(&read_requirements(), &profile_facts());

    assert_eq!(report.status, LowerabilityStatus::Native);
    assert!(report.failures.is_empty());
    assert_eq!(report.effect_results.len(), 1);
    assert!(report.effect_results[0].is_native());
}

#[test]
fn one_direct_adapter_satisfies_v1_lowering_requirements() {
    let mut facts = profile_facts();
    facts.native_effects.clear();
    facts.direct_adapters.push(direct_adapter());

    let report = check_lowerability(&read_requirements(), &facts);

    assert_eq!(report.status, LowerabilityStatus::Adapted);
    assert!(report.failures.is_empty());
    assert_eq!(
        report.effect_results[0].adapter_coordinate(),
        Some("hello.optics@1.kv.transactional.adapter/v1")
    );
}

#[test]
fn missing_native_or_adapter_support_is_unsupported() {
    let mut facts = profile_facts();
    facts.native_effects.clear();

    let report = check_lowerability(&read_requirements(), &facts);

    assert_eq!(report.status, LowerabilityStatus::Unsupported);
    assert_eq!(
        report
            .failures
            .iter()
            .map(|failure| failure.kind)
            .collect::<Vec<_>>(),
        vec![LowerabilityFailureKind::MissingEffectSupport]
    );
}

#[test]
fn v1_rejects_chained_adapter_claims() {
    let mut facts = profile_facts();
    facts.native_effects.clear();
    let mut adapter = direct_adapter();
    adapter
        .emits_semantic_effects
        .push("other.lawpack@1.remainingEffect".to_owned());
    facts.direct_adapters.push(adapter);

    let report = check_lowerability(&read_requirements(), &facts);

    assert_eq!(report.status, LowerabilityStatus::Unsupported);
    assert_eq!(
        report
            .failures
            .iter()
            .map(|failure| failure.kind)
            .collect::<Vec<_>>(),
        vec![LowerabilityFailureKind::ChainedAdapterUnsupported]
    );
}

#[test]
fn v1_rejects_ambiguous_direct_adapters() {
    let mut facts = profile_facts();
    facts.native_effects.clear();
    facts.direct_adapters.push(direct_adapter());
    let mut second = direct_adapter();
    second.adapter.coordinate = "hello.optics@1.kv.transactional.adapter.alt/v1".to_owned();
    facts.direct_adapters.push(second);

    let report = check_lowerability(&read_requirements(), &facts);

    assert_eq!(report.status, LowerabilityStatus::Unsupported);
    assert_eq!(
        report
            .failures
            .iter()
            .map(|failure| failure.kind)
            .collect::<Vec<_>>(),
        vec![LowerabilityFailureKind::AmbiguousAdapter]
    );
}
