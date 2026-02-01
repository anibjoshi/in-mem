//! Audit test for issue #955: KvGet/StateRead/JsonGet strip version metadata
//! Verdict: CONFIRMED BUG
//!
//! The get-by-key operations for KV, State, and JSON all return
//! `Output::Maybe(Option<Value>)` instead of `Output::MaybeVersioned(Option<VersionedValue>)`.
//! This means the version number and timestamp associated with the stored value
//! are stripped and lost on read.
//!
//! - kv.rs:49     — `Ok(Output::Maybe(result))`
//! - state.rs:55  — `Ok(Output::Maybe(result))`
//! - json.rs:94   — `Ok(Output::Maybe(mapped))`
//!
//! Meanwhile, KvPut/StateSet/JsonSet all return `Output::Version(...)`, so the
//! version is available at write time. But after a write, a subsequent read
//! cannot retrieve that version number. The only way to get versions back is
//! to use the version-history commands (KvGetv, StateReadv, JsonGetv).

use strata_core::value::Value;
use strata_engine::database::Database;
use strata_executor::{BranchId, Command, Executor, Output};

/// KvGet strips version metadata from the returned value.
#[test]
fn issue_955_kvget_strips_version() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Store a value and capture the version
    let put_result = executor
        .execute(Command::KvPut {
            branch: Some(branch.clone()),
            key: "k1".into(),
            value: Value::Int(42),
        })
        .unwrap();
    let _version = match put_result {
        Output::Version(v) => v,
        other => panic!("KvPut should return Version, got: {:?}", other),
    };

    // Get it back — version is lost
    let get_result = executor
        .execute(Command::KvGet {
            branch: Some(branch.clone()),
            key: "k1".into(),
        })
        .unwrap();

    // BUG: Returns Maybe instead of MaybeVersioned — version metadata stripped
    assert!(
        matches!(get_result, Output::Maybe(_)),
        "KvGet strips version info, returning Maybe instead of MaybeVersioned. Got: {:?}",
        get_result
    );
}

/// StateRead strips version metadata from the returned value.
#[test]
fn issue_955_state_read_strips_version() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Initialize a state cell
    let init_result = executor
        .execute(Command::StateInit {
            branch: Some(branch.clone()),
            cell: "cell1".into(),
            value: Value::Int(1),
        })
        .unwrap();
    let _version = match init_result {
        Output::Version(v) => v,
        other => panic!("StateInit should return Version, got: {:?}", other),
    };

    // Read the state cell — version is lost
    let state_result = executor
        .execute(Command::StateRead {
            branch: Some(branch.clone()),
            cell: "cell1".into(),
        })
        .unwrap();

    // BUG: Returns Maybe instead of MaybeVersioned
    assert!(
        matches!(state_result, Output::Maybe(_)),
        "StateRead strips version info, returning Maybe instead of MaybeVersioned. Got: {:?}",
        state_result
    );
}

/// JsonGet strips version metadata from the returned value.
#[test]
fn issue_955_json_get_strips_version() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Create a JSON document
    let set_result = executor
        .execute(Command::JsonSet {
            branch: Some(branch.clone()),
            key: "doc1".into(),
            path: "$".into(),
            value: Value::Int(99),
        })
        .unwrap();
    let _version = match set_result {
        Output::Version(v) => v,
        other => panic!("JsonSet should return Version, got: {:?}", other),
    };

    // Read the JSON document — version is lost
    let get_result = executor
        .execute(Command::JsonGet {
            branch: Some(branch.clone()),
            key: "doc1".into(),
            path: "$".into(),
        })
        .unwrap();

    // BUG: Returns Maybe instead of MaybeVersioned
    assert!(
        matches!(get_result, Output::Maybe(_)),
        "JsonGet strips version info, returning Maybe instead of MaybeVersioned. Got: {:?}",
        get_result
    );
}

/// Contrast: the version-history commands (KvGetv, StateReadv) DO preserve versions.
#[test]
fn issue_955_version_history_commands_do_preserve_versions() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Write a KV entry
    executor
        .execute(Command::KvPut {
            branch: Some(branch.clone()),
            key: "k2".into(),
            value: Value::Int(10),
        })
        .unwrap();

    // KvGetv preserves version history
    let getv_result = executor
        .execute(Command::KvGetv {
            branch: Some(branch.clone()),
            key: "k2".into(),
        })
        .unwrap();

    match getv_result {
        Output::VersionHistory(Some(versions)) => {
            assert!(!versions.is_empty(), "Should have at least one version");
            // Each VersionedValue has .version and .timestamp fields
            let first = &versions[0];
            assert!(first.version > 0, "Version should be set");
        }
        Output::VersionHistory(None) => {
            panic!("Key should exist in version history");
        }
        other => panic!("Expected VersionHistory, got: {:?}", other),
    }
}
