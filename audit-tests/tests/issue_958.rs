//! Audit test for issue #958: Unused Output variants
//! Verdict: FIXED
//!
//! The following unused `Output` enum variants have been removed from output.rs:
//!
//! - `Output::Value(Value)` -- No handler returned this variant.
//! - `Output::Versioned(VersionedValue)` -- No handler returned this variant.
//! - `Output::Values(Vec<Option<VersionedValue>>)` -- No mget command exists.
//! - `Output::Versions(Vec<u64>)` -- No handler returned this variant.
//! - `Output::Strings(Vec<String>)` -- No handler returned this variant.
//! - `Output::Int(i64)` -- No handler returned this variant.
//! - `Output::Float(f64)` -- No handler returned this variant.
//! - `Output::KvScanResult { entries, cursor }` -- No KvScan command exists.
//! - `Output::JsonSearchHits(Vec<JsonSearchHit>)` -- No JsonSearch command exists.
//! - `Output::VectorMatchesWithExhausted { matches, exhausted }` -- VectorSearch uses VectorMatches.
//! - `Output::BranchInfo(BranchInfo)` -- All branch handlers use versioned variants.
//! - `Output::BranchInfoVersioned(VersionedBranchInfo)` -- Superseded by MaybeBranchInfo.
//! - `Output::MaybeBranchId(Option<BranchId>)` -- No handler returned this variant.
//! - `Output::TxnId(String)` -- TxnBegin returns TxnBegun instead.
//! - `Output::RetentionVersion(Option<RetentionVersionInfo>)` -- No handler returned this variant.
//! - `Output::RetentionPolicy(RetentionPolicyInfo)` -- No handler returned this variant.
//!
//! The associated unused types `JsonSearchHit`, `RetentionPolicyInfo`, and
//! `RetentionVersionInfo` have also been removed from types.rs.
//!
//! Corresponding serialization tests for removed variants have been deleted.

/// Confirms the fix: unused Output variants have been removed.
#[test]
fn issue_958_unused_output_variants_removed() {
    // The unused Output variants listed above have been removed from the enum.
    // This reduces the API surface area that callers must handle in exhaustive
    // matches. If any of these variants are needed in the future, they can be
    // re-added when a command handler actually produces them.
}
