//! File-backed authority-facts loading behavior.
//!
//! These tests assert compiler behavior and stable load failure kinds. They do
//! not assert documentation text, repository layout, or implementation details.

use std::fs;
use std::path::{Path, PathBuf};

use edict_syntax::{
    compile_to_core, load_compiler_context_from_authority_fact_files,
    AuthorityFactsLoadFailureKind, CompilerErrorKind, CompilerStage,
};

const BOUNDED_HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");

#[test]
fn file_backed_authority_facts_compile_bounded_hello() {
    let dir = temp_case_dir("compile-bounded-hello");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        target_profile_facts("read"),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", lawpack_budget_facts());
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module = edict_syntax::parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &context).expect("loaded facts compile fixture");
    let intent = core.intents.get("sayHello").expect("compiled intent");

    assert_eq!(
        intent.required_operation_profile,
        "continuum.profile.read-only/v1"
    );
    assert_eq!(intent.core_evaluation_budget.max_steps, 64);
    assert_eq!(intent.core_evaluation_budget.max_allocated_bytes, 4096);
    assert_eq!(intent.core_evaluation_budget.max_output_bytes, 1024);
}

#[test]
fn file_backed_authority_facts_reject_write_effect_profile_mismatch() {
    let dir = temp_case_dir("profile-effect-mismatch");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        target_profile_facts("read"),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", lawpack_effect_facts("replace"));
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module = edict_syntax::parse_module(
        "package a.b@1;\n\
         type Input = { id: String<max=16>, };\n\
         type Output = { id: String<max=16>, };\n\
         intent t(input: Input) returns Output\n\
           profile p.readOnly\n\
           basis none\n\
           budget <= p.tiny {\n\
           target.replace(input.id) else domain.WriteRejected;\n\
           return { id: input.id };\n\
         }",
    )
    .expect("source parses");

    let errors = compile_to_core(&module, &context)
        .expect_err("loaded write-class facts reject incompatible effect");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert_eq!(
        errors
            .iter()
            .map(|err| err.kind)
            .collect::<Vec<CompilerErrorKind>>(),
        vec![CompilerErrorKind::ProfileEffectMismatch]
    );
}

#[test]
fn malformed_authority_facts_file_rejects_with_stable_kind() {
    let dir = temp_case_dir("malformed-json");
    let path = write_json(&dir, "bad.json", "{");
    let failures =
        load_compiler_context_from_authority_fact_files([path.as_path()]).expect_err("bad JSON");

    assert_eq!(
        failure_kinds(&failures),
        vec![AuthorityFactsLoadFailureKind::InvalidJson]
    );
}

#[test]
fn nondigest_authority_fact_source_rejects_with_stable_kind() {
    let dir = temp_case_dir("nondigest-source");
    let path = write_json(
        &dir,
        "floating-source.json",
        r#"{
          "apiVersion": "edict.authority-facts/v1",
          "source": {
            "kind": "targetProfile",
            "coordinate": "echo.dpo@1",
            "digest": null
          },
          "operationProfiles": [],
          "effectWriteClasses": [],
          "budgets": []
        }"#,
    );
    let failures = load_compiler_context_from_authority_fact_files([path.as_path()])
        .expect_err("floating source rejects");

    assert_eq!(
        failure_kinds(&failures),
        vec![AuthorityFactsLoadFailureKind::NonDigestLockedSource]
    );
}

#[test]
fn conflicting_file_backed_authority_facts_reject_before_context() {
    let dir = temp_case_dir("conflicting-facts");
    let read = write_json(&dir, "read-effect.json", lawpack_effect_facts("read"));
    let replace = write_json(&dir, "replace-effect.json", lawpack_effect_facts("replace"));
    let failures =
        load_compiler_context_from_authority_fact_files([read.as_path(), replace.as_path()])
            .expect_err("conflicting facts reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![AuthorityFactsLoadFailureKind::ConflictingFact]
    );
}

#[test]
fn mixed_authority_source_digests_reject_before_context() {
    let dir = temp_case_dir("mixed-source-digests");
    let first = write_json(
        &dir,
        "target-profile-a.json",
        target_profile_facts_with_digest(
            "read",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        ),
    );
    let second = write_json(
        &dir,
        "target-profile-b.json",
        target_profile_facts_with_digest(
            "read",
            "sha256:4444444444444444444444444444444444444444444444444444444444444444",
        ),
    );
    let failures =
        load_compiler_context_from_authority_fact_files([first.as_path(), second.as_path()])
            .expect_err("mixed source digests reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![AuthorityFactsLoadFailureKind::ConflictingFact]
    );
}

fn temp_case_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "edict-authority-facts-{name}-{}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("remove stale temp authority-facts directory");
    }
    fs::create_dir_all(&dir).expect("create temp authority-facts directory");
    dir
}

fn write_json(dir: &Path, name: &str, contents: impl AsRef<str>) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents.as_ref()).expect("write authority-facts JSON");
    path
}

fn target_profile_facts(allowed_write_class: &str) -> String {
    target_profile_facts_with_digest(
        allowed_write_class,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
}

fn target_profile_facts_with_digest(allowed_write_class: &str, digest: &str) -> String {
    format!(
        r#"{{
          "apiVersion": "edict.authority-facts/v1",
          "source": {{
            "kind": "targetProfile",
            "coordinate": "echo.dpo@1",
            "digest": "{digest}"
          }},
          "operationProfiles": [
            {{
              "source": "hello.readOnly",
              "core": "continuum.profile.read-only/v1",
              "allowedWriteClasses": ["{allowed_write_class}"]
            }},
            {{
              "source": "p.readOnly",
              "core": "continuum.profile.read-only/v1",
              "allowedWriteClasses": ["{allowed_write_class}"]
            }}
          ],
          "effectWriteClasses": [],
          "budgets": []
        }}"#
    )
}

fn lawpack_budget_facts() -> &'static str {
    r#"{
      "apiVersion": "edict.authority-facts/v1",
      "source": {
        "kind": "lawpack",
        "coordinate": "hello.optics@1",
        "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222"
      },
      "operationProfiles": [],
      "effectWriteClasses": [],
      "budgets": [
        {
          "source": "hello.tinyBudget",
          "maxSteps": 64,
          "maxAllocatedBytes": 4096,
          "maxOutputBytes": 1024
        }
      ]
    }"#
}

fn lawpack_effect_facts(write_class: &str) -> String {
    format!(
        r#"{{
          "apiVersion": "edict.authority-facts/v1",
          "source": {{
            "kind": "lawpack",
            "coordinate": "hello.optics@1",
            "digest": "sha256:3333333333333333333333333333333333333333333333333333333333333333"
          }},
          "operationProfiles": [],
          "effectWriteClasses": [
            {{
              "effect": "target.replace",
              "writeClass": "{write_class}"
            }}
          ],
          "budgets": [
            {{
              "source": "p.tiny",
              "maxSteps": 1,
              "maxAllocatedBytes": 1,
              "maxOutputBytes": 1
            }}
          ]
        }}"#
    )
}

fn failure_kinds(
    failures: &[edict_syntax::AuthorityFactsLoadFailure],
) -> Vec<AuthorityFactsLoadFailureKind> {
    failures.iter().map(|failure| failure.kind).collect()
}
