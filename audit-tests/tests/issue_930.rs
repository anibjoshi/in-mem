//! Audit test for issue #930: VersionConflict loses version type information
//! Verdict: FIXED
//!
//! The `Error::VersionConflict` variant now includes `expected_type` and
//! `actual_type` fields that preserve the Version enum variant name
//! (e.g., "Counter", "Txn", "Sequence"). This allows distinguishing
//! conflicts between different version types.

use strata_core::{EntityRef, StrataError, Version};

/// Verifies that version type is now preserved during conversion.
/// Counter(5) and Txn(5) are distinguishable via the type fields.
#[test]
fn issue_930_version_type_preserved_on_conversion() {
    let entity = EntityRef::kv(strata_core::types::BranchId::from_bytes([0; 16]), "key1");

    // Conflict between Counter(5) and Txn(5) — different types, same number
    let err = StrataError::version_conflict(entity, Version::Counter(5), Version::Txn(5));

    let exec_err: strata_executor::Error = err.into();

    match exec_err {
        strata_executor::Error::VersionConflict {
            expected,
            actual,
            expected_type,
            actual_type,
        } => {
            assert_eq!(expected, 5);
            assert_eq!(actual, 5);
            // Type information is now preserved
            assert_eq!(expected_type, "Counter");
            assert_eq!(actual_type, "Txn");
        }
        other => panic!("Expected VersionConflict, got: {:?}", other),
    }
}

/// Normal version conflict where numeric values differ.
#[test]
fn issue_930_normal_conflict_values_differ() {
    let entity = EntityRef::kv(strata_core::types::BranchId::from_bytes([0; 16]), "key1");

    let err = StrataError::version_conflict(entity, Version::Counter(3), Version::Counter(7));

    let exec_err: strata_executor::Error = err.into();

    match exec_err {
        strata_executor::Error::VersionConflict {
            expected,
            actual,
            expected_type,
            actual_type,
        } => {
            assert_eq!(expected, 3);
            assert_eq!(actual, 7);
            assert_eq!(expected_type, "Counter");
            assert_eq!(actual_type, "Counter");
        }
        other => panic!("Expected VersionConflict, got: {:?}", other),
    }
}

/// Sequence and Txn versions are distinguishable via type fields.
#[test]
fn issue_930_sequence_and_txn_distinguishable() {
    let entity = EntityRef::kv(strata_core::types::BranchId::from_bytes([0; 16]), "key1");

    // Sequence(10) vs Txn(20)
    let err = StrataError::version_conflict(entity, Version::Sequence(10), Version::Txn(20));

    let exec_err: strata_executor::Error = err.into();

    match exec_err {
        strata_executor::Error::VersionConflict {
            expected,
            actual,
            expected_type,
            actual_type,
        } => {
            assert_eq!(expected, 10);
            assert_eq!(actual, 20);
            assert_eq!(expected_type, "Sequence");
            assert_eq!(actual_type, "Txn");
        }
        other => panic!("Expected VersionConflict, got: {:?}", other),
    }
}
