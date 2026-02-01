//! Audit test for issue #935: NotFound routing uses string prefix parsing
//! Verdict: ARCHITECTURAL CHOICE
//!
//! In convert.rs, `StrataError::NotFound { entity_ref }` is routed to
//! different executor error variants based on string prefix matching:
//!
//! ```ignore
//! let entity_str = entity_ref.to_string();
//! if entity_str.starts_with("kv:") || entity_str.starts_with("json:") {
//!     Error::KeyNotFound { key: entity_str }
//! } else if entity_str.starts_with("branch:") {
//!     Error::BranchNotFound { branch: entity_str }
//! } else if entity_str.starts_with("collection:") || entity_str.starts_with("vector:") {
//!     Error::CollectionNotFound { collection: entity_str }
//! } else if entity_str.starts_with("stream:") || entity_str.starts_with("event:") {
//!     Error::StreamNotFound { stream: entity_str }
//! } else if entity_str.starts_with("state:") || entity_str.starts_with("cell:") {
//!     Error::CellNotFound { cell: entity_str }
//! } else {
//!     Error::KeyNotFound { key: entity_str }
//! }
//! ```
//!
//! This string-prefix-based routing is fragile:
//! 1. If EntityRef::Display changes format, the routing breaks silently
//! 2. Unrecognized prefixes fall through to KeyNotFound (may be wrong)
//! 3. The full entity_str (including prefix) is passed as the key/branch/etc.
//!
//! A more robust approach would be to use a typed enum or match on the
//! EntityRef variant directly, rather than its string representation.

use strata_core::{EntityRef, StrataError};

/// Demonstrates that NotFound routing depends on string prefix of entity_ref.
#[test]
fn issue_935_not_found_uses_string_prefix() {
    let branch_id = strata_core::types::BranchId::from_bytes([0; 16]);

    // KV entity -> KeyNotFound
    let err: strata_executor::Error =
        StrataError::not_found(EntityRef::kv(branch_id, "mykey")).into();
    assert!(
        matches!(err, strata_executor::Error::KeyNotFound { .. }),
        "kv: prefix should map to KeyNotFound, got: {:?}",
        err
    );

    // State entity -> CellNotFound
    let err: strata_executor::Error =
        StrataError::not_found(EntityRef::state(branch_id, "mycell")).into();
    assert!(
        matches!(err, strata_executor::Error::CellNotFound { .. }),
        "state: prefix should map to CellNotFound, got: {:?}",
        err
    );
}

/// Demonstrates that the full entity_ref string (including prefix) is
/// included in the error, not just the user-facing key name.
#[test]
fn issue_935_error_contains_full_entity_ref_string() {
    let branch_id = strata_core::types::BranchId::from_bytes([0; 16]);

    let err: strata_executor::Error =
        StrataError::not_found(EntityRef::kv(branch_id, "userkey")).into();

    match err {
        strata_executor::Error::KeyNotFound { key } => {
            // The key field contains the full entity_ref string, including
            // the "kv:" prefix and branch ID — not just the user key
            assert!(
                key.starts_with("kv:"),
                "Key should include the kv: prefix, got: {}",
                key
            );
            assert!(
                key.contains("userkey"),
                "Key should contain the user key, got: {}",
                key
            );
        }
        other => panic!("Expected KeyNotFound, got: {:?}", other),
    }
}
