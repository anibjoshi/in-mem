//! Audit test for issue #962: WAL recovery stops at first error
//! Verdict: ARCHITECTURAL CHOICE
//!
//! During WAL (Write-Ahead Log) recovery/replay, the replay loop processes
//! entries sequentially. If it encounters a corrupted or unreadable entry,
//! it stops replaying immediately. All entries after the corrupted one are
//! lost, even if they are perfectly valid.
//!
//! Alternative strategies that are NOT implemented:
//! - Skip the corrupted entry and continue replaying subsequent entries
//! - Scan forward to find the next valid entry boundary and resume
//! - Replay all valid entries and report which ones were skipped
//!
//! This is classified as an ARCHITECTURAL CHOICE because:
//! 1. WAL entries may have causal dependencies — skipping one entry could
//!    leave the database in an inconsistent state (e.g., a delete followed
//!    by a re-create: skipping the delete would show stale data)
//! 2. The WAL is append-only and sequentially written, so corruption in
//!    the middle usually indicates a crash during write. Entries after the
//!    crash point were never fully committed and should not be replayed.
//! 3. Stop-at-first-error provides a clear invariant: all replayed entries
//!    are sequential and valid, with no gaps
//! 4. CRC checking on each entry means a corrupt entry is reliably detected

/// Documents the architectural choice regarding WAL recovery behavior.
/// Testing actual WAL corruption requires direct file manipulation
/// outside the executor API scope.
#[test]
fn issue_962_wal_recovery_stops_at_first_error_documented() {
    // WAL replay loop (simplified):
    //
    //   for entry in wal_reader.entries() {
    //       match entry {
    //           Ok(e) => apply(e),
    //           Err(_) => break,  // <-- stops here, remaining entries lost
    //       }
    //   }
    //
    // If entry N is corrupt, entries N+1, N+2, ... are not replayed even
    // if they have valid CRCs.
    //
    // Justification:
    // - Sequential consistency: no gaps in the replay sequence
    // - Crash semantics: entries after the corruption point may be
    //   from an incomplete write and should not be trusted
    // - Deterministic recovery: same corrupt WAL always produces
    //   the same recovered state
    //
    // ARCHITECTURAL CHOICE: Stop-at-first-error provides the strongest
    // consistency guarantee. Skip-and-continue would require proving that
    // each entry is independent, which is not true for all operations.
}
