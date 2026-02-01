//! Audit test for issue #958: Unused Output variants
//! Verdict: ARCHITECTURAL CHOICE
//!
//! Several `Output` enum variants are never produced by any handler in production
//! code. These variants exist in the Output enum (output.rs) but no command
//! handler ever constructs them:
//!
//! - `Output::Value(Value)` — Single value without version info.
//!   No handler returns this variant. Get operations return `Maybe` or
//!   `MaybeVersioned`, not bare `Value`.
//!
//! - `Output::Versioned(VersionedValue)` — Value with version metadata.
//!   No handler returns this. KvGet returns `Maybe`, not `Versioned`.
//!
//! - `Output::MaybeVersioned(Option<VersionedValue>)` — Optional versioned value.
//!   Despite being documented as the return type for KvGet, no handler actually
//!   constructs it. Only the in-transaction Session path uses `MaybeVersioned`
//!   for EventRead.
//!
//! - `Output::Values(Vec<Option<VersionedValue>>)` — Multiple optional versioned values.
//!   Intended for mget-style operations, but no mget command exists yet.
//!
//! - `Output::Versions(Vec<u64>)` — List of versions.
//!   No handler constructs this variant.
//!
//! - `Output::Strings(Vec<String>)` — List of strings.
//!   No handler constructs this variant.
//!
//! - `Output::Int(i64)` — Signed integer result.
//!   Intended for increment operations, but json_increment is not in MVP.
//!
//! - `Output::Float(f64)` — Float result.
//!   Intended for json_increment float mode, not implemented.
//!
//! - `Output::KvScanResult { entries, cursor }` — KV scan with cursor.
//!   No KvScan command exists in the Command enum.
//!
//! - `Output::JsonSearchHits(Vec<JsonSearchHit>)` — JSON search hits.
//!   No JsonSearch command exists in the Command enum.
//!
//! - `Output::VectorMatchesWithExhausted { matches, exhausted }` — Vector search
//!   with budget exhaustion flag. VectorSearch returns `VectorMatches` instead.
//!
//! - `Output::BranchInfo(BranchInfo)` — Unversioned branch info.
//!   All branch handlers use `BranchInfoVersioned` or `BranchWithVersion`.
//!
//! - `Output::MaybeBranchId(Option<BranchId>)` — Optional branch ID.
//!   No handler returns this variant.
//!
//! - `Output::TxnId(String)` — Transaction ID.
//!   TxnBegin returns `TxnBegun`, not `TxnId`.
//!
//! - `Output::TxnCommitted { version }` — Transaction committed with version.
//!   Session's TxnCommit returns `Output::Version`, not `TxnCommitted`.
//!
//! - `Output::TxnAborted` — Transaction aborted.
//!   Session's TxnRollback returns `Output::Unit`, not `TxnAborted`.
//!
//! These unused variants add surface area to the Output enum that callers must
//! handle in exhaustive matches, even though they will never be produced.
//! This is an architectural choice — the variants exist as forward declarations
//! for planned features or to maintain API symmetry.

/// Documents the architectural choice. No runtime test is needed since this is
/// about unreachable code paths in the Output enum. Verify by inspecting the
/// handler implementations in handlers/*.rs.
#[test]
fn issue_958_unused_output_variants_documented() {
    // This test documents that the following Output variants are never
    // constructed by any handler. The enum carries them as forward
    // declarations for future features or API completeness.
    //
    // A grep for each variant in handlers/ confirms they are unused:
    //   Output::Value        — 0 handler usages
    //   Output::Versioned    — 0 handler usages
    //   Output::Values       — 0 handler usages
    //   Output::Versions     — 0 handler usages
    //   Output::Strings      — 0 handler usages
    //   Output::Int          — 0 handler usages
    //   Output::Float        — 0 handler usages
    //   Output::KvScanResult — 0 handler usages
    //   Output::BranchInfo   — 0 handler usages
    //   Output::TxnId        — 0 handler usages
    //   Output::TxnCommitted — 0 handler usages
    //   Output::TxnAborted   — 0 handler usages
    //
    // ARCHITECTURAL CHOICE: Keeping unused variants is a design decision
    // that trades a larger enum for forward compatibility and API symmetry.
}
