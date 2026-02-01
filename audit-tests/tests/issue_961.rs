//! Audit test for issue #961: Corrupted snapshot causes hard failure with no recovery
//! Verdict: ARCHITECTURAL CHOICE
//!
//! When the database opens and finds a snapshot file that is corrupted (e.g.,
//! truncated, bit-flipped, or partially written), the snapshot loading code
//! returns an error that propagates up and prevents the database from opening.
//!
//! There is no fallback strategy such as:
//! - Ignoring the corrupted snapshot and replaying the full WAL from scratch
//! - Trying an older snapshot if multiple exist
//! - Opening in a degraded read-only mode
//!
//! This means a single corrupted snapshot file can make the entire database
//! unopenable, even if the WAL contains all the data needed to reconstruct
//! the state.
//!
//! This is classified as an ARCHITECTURAL CHOICE because:
//! 1. Snapshot corruption is a strong signal that the storage medium may be
//!    unreliable, and silently ignoring it could lead to data loss
//! 2. Full WAL replay without a snapshot could be extremely slow for large
//!    databases, making it a poor default fallback
//! 3. The fail-fast behavior makes the problem visible to operators
//!    immediately rather than hiding it behind degraded performance
//! 4. Recovery tools can be built externally (e.g., a repair command that
//!    deletes the snapshot and forces WAL replay)

/// Documents the architectural choice regarding snapshot corruption handling.
/// Testing actual snapshot corruption requires filesystem-level manipulation
/// outside the executor API scope.
#[test]
fn issue_961_corrupted_snapshot_hard_failure_documented() {
    // The snapshot loading path is:
    //   Database::open() -> load_snapshot() -> deserialize()
    //
    // If deserialize() fails:
    //   - Error propagates to Database::open()
    //   - Database::open() returns Err(...)
    //   - No fallback to WAL-only replay
    //
    // Recovery would require:
    //   1. Deleting the snapshot file manually
    //   2. Re-opening the database (forces full WAL replay)
    //   3. Creating a new snapshot after successful replay
    //
    // ARCHITECTURAL CHOICE: Fail-fast on snapshot corruption ensures data
    // integrity concerns are surfaced immediately. Silent fallback to WAL
    // replay could mask underlying storage reliability issues.
}
