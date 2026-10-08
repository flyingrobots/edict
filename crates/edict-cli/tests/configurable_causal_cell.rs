#[path = "../examples/configurable_causal_cell/limits.rs"]
mod limits;

use edict_syntax::{encode_canonical_cbor, CanonicalValue};
use limits::{LimitError, Limits};

#[test]
fn configurable_cell_budgets_match_maximum_unicode_encoding() {
    for (key_scalars, value_scalars, replacement_bytes) in [
        (64, 64, 256),
        (64, 1024, 4096),
        (1, 5, 20),
        (6, 6, 24),
        (64, 16384, 65536),
    ] {
        let limits = Limits {
            key_scalars,
            value_scalars,
            replacement_bytes,
        };
        let budget = limits.budgets().expect("consistent finite parameters");
        let value = CanonicalValue::Map(vec![
            (
                CanonicalValue::Text("key".into()),
                CanonicalValue::Text("😀".repeat(usize::try_from(key_scalars).unwrap())),
            ),
            (
                CanonicalValue::Text("value".into()),
                CanonicalValue::Text("😀".repeat(usize::try_from(value_scalars).unwrap())),
            ),
        ]);
        let encoded = encode_canonical_cbor(&value).unwrap();
        assert_eq!(budget.output, u64::try_from(encoded.len()).unwrap());
        assert_eq!(budget.write, replacement_bytes + 64);
        assert!(budget.allocated >= budget.output + replacement_bytes);
        assert_eq!(limits.budgets(), Ok(budget), "calculation is deterministic");
    }
}

#[test]
fn configurable_cell_limits_reject_inconsistent_and_overflowing_parameters() {
    for limits in [
        Limits {
            key_scalars: 0,
            value_scalars: 64,
            replacement_bytes: 256,
        },
        Limits {
            key_scalars: 64,
            value_scalars: 0,
            replacement_bytes: 256,
        },
        Limits {
            key_scalars: 64,
            value_scalars: 64,
            replacement_bytes: 0,
        },
    ] {
        assert_eq!(limits.budgets(), Err(LimitError::Zero));
    }
    assert_eq!(
        Limits {
            key_scalars: 64,
            value_scalars: 256,
            replacement_bytes: 256
        }
        .budgets(),
        Err(LimitError::ScalarBytesExceedCap)
    );
    for limits in [
        Limits {
            key_scalars: u64::MAX,
            value_scalars: 1,
            replacement_bytes: 4,
        },
        Limits {
            key_scalars: 1,
            value_scalars: u64::MAX,
            replacement_bytes: u64::MAX,
        },
        Limits {
            key_scalars: 1,
            value_scalars: 1,
            replacement_bytes: u64::MAX,
        },
    ] {
        assert_eq!(limits.budgets(), Err(LimitError::Overflow));
    }
}
