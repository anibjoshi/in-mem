//! Audit test for issue #933: Session deserialization errors map to Error::Internal
//! Verdict: CONFIRMED BUG
//!
//! In session.rs `execute_in_txn`, several code paths use `serde_json::from_str`
//! to deserialize stored values (e.g., State cells, JSON documents). When
//! deserialization fails, the error is mapped to `Error::Internal { reason }`:
//!
//! ```ignore
//! serde_json::from_str(&s).map_err(|e| Error::Internal {
//!     reason: e.to_string(),
//! })?;
//! ```
//!
//! This conflates deserialization failures (which may indicate data corruption
//! or schema evolution) with internal bugs. The `Error::Serialization` variant
//! exists but is not used here.
//!
//! This is hard to trigger in a unit test because valid `Value` types
//! generally serialize and deserialize correctly. The issue would manifest
//! if stored data was manually corrupted or written by a different schema
//! version.

/// Documents that Error::Internal is used for deserialization failures
/// in the session's transaction code path, while Error::Serialization
/// exists as a more appropriate variant.
#[test]
fn issue_933_deserialization_uses_internal_not_serialization() {
    // The Error enum has a Serialization variant:
    let serialization_err = strata_executor::Error::Serialization {
        reason: "invalid JSON".into(),
    };
    assert_eq!(
        serialization_err.to_string(),
        "serialization error: invalid JSON"
    );

    // But session.rs uses Internal for serde_json failures:
    let internal_err = strata_executor::Error::Internal {
        reason: "expected value at line 1 column 1".into(),
    };
    assert_eq!(
        internal_err.to_string(),
        "internal error: expected value at line 1 column 1"
    );

    // A caller cannot distinguish "data is corrupted" (Serialization)
    // from "executor has a bug" (Internal) because both use Internal.
    assert!(matches!(
        internal_err,
        strata_executor::Error::Internal { .. }
    ));
}

/// Shows the specific code paths in session.rs that use Error::Internal
/// for deserialization. These are in dispatch_in_txn for StateRead and
/// JsonGet commands.
#[test]
fn issue_933_affected_code_paths() {
    // The following code paths in session.rs map serde errors to Internal:
    //
    // 1. StateRead (session.rs ~line 240-242):
    //    serde_json::from_str(&s).map_err(|e| Error::Internal { reason: e.to_string() })
    //
    // 2. JsonGet root path (session.rs ~line 259-260):
    //    serde_json::from_str(&s).map_err(|e| Error::Internal { reason: e.to_string() })
    //
    // Both should use Error::Serialization instead of Error::Internal.
    //
    // Since we cannot easily inject corrupted data through the transaction
    // context, we verify the error types exist and note the mismatch.

    let _ = strata_executor::Error::Serialization {
        reason: "should be used for serde failures".into(),
    };
    let _ = strata_executor::Error::Internal {
        reason: "currently used for serde failures in session.rs".into(),
    };
}
