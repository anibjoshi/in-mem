//! Audit test for issue #945: VersionChain GC never invoked
//! Verdict: ARCHITECTURAL CHOICE
//!
//! The `VersionChain` type in `strata-storage` has a public `gc(min_version: u64)` method
//! that removes all versions older than `min_version`. However, this method is never
//! called in production code.
//!
//! Evidence:
//! - `VersionChain::gc()` is defined and tested in strata-storage
//! - No production code path calls gc() on any VersionChain
//! - The RetentionApply command returns "not yet implemented"
//! - There is no background GC thread or compaction process
//!
//! This means version chains grow unboundedly for the lifetime of the database.
//! Every write to a key appends a new version, and old versions are never reclaimed.
//!
//! Impact:
//! - Memory usage grows linearly with the number of writes (not just unique keys)
//! - A key written 1 million times will have 1 million versions in its chain
//! - This is acceptable for short-lived ephemeral databases but problematic for
//!   long-running persistent databases
//!
//! The fix requires implementing a GC trigger (e.g., RetentionApply command,
//! background thread, or write-count threshold) and integrating it with the
//! snapshot pinning mechanism (see issue #903).

use strata_core::value::Value;
use strata_engine::database::Database;
use strata_executor::BranchId;
use strata_executor::{Command, Executor, Output};

/// Demonstrates that version chains grow without bound because GC is never invoked.
///
/// We write to the same key many times and verify that:
/// 1. Each write creates a new version (version numbers increase)
/// 2. The version history is available (old versions are retained)
/// 3. There is no mechanism to trigger GC through the public API
#[test]
fn issue_945_version_chain_gc_never_invoked() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Write to the same key many times to grow its version chain
    let num_writes = 20;
    let mut versions = Vec::new();

    for i in 0..num_writes {
        let result = executor
            .execute(Command::KvPut {
                branch: Some(branch.clone()),
                key: "frequently_updated".into(),
                value: Value::Int(i),
            })
            .unwrap();

        if let Output::Version(v) = result {
            versions.push(v);
        }
    }

    // Versions should be monotonically increasing
    for window in versions.windows(2) {
        assert!(
            window[1] > window[0],
            "Versions should be monotonically increasing"
        );
    }

    // RetentionApply (which would trigger GC) is not implemented
    let retention_result = executor.execute(Command::RetentionApply {
        branch: Some(branch.clone()),
    });

    assert!(
        retention_result.is_err(),
        "RetentionApply should return 'not yet implemented' error, \
         confirming that GC cannot be triggered through the public API"
    );

    // The version history should contain all versions (nothing was GC'd)
    let history_result = executor
        .execute(Command::KvGetv {
            branch: Some(branch.clone()),
            key: "frequently_updated".into(),
        })
        .unwrap();

    match history_result {
        Output::VersionHistory(Some(history)) => {
            // All versions should be present since GC never runs
            assert!(
                history.len() >= 2,
                "Version history should contain multiple versions (GC never runs). \
                 Got {} versions.",
                history.len()
            );
        }
        Output::VersionHistory(None) => {
            panic!("Key should exist and have version history");
        }
        other => panic!("Expected VersionHistory, got {:?}", other),
    }
}
