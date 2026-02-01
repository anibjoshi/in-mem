//! Audit test for issue #954: Output docstring says KvGet returns MaybeVersioned but it
//! returns Maybe
//! Verdict: CONFIRMED BUG (documentation)
//!
//! The `Output` enum docstring (output.rs) shows an example where KvGet returns
//! `Output::MaybeVersioned`, and the `Command::KvGet` variant docstring in
//! command.rs says "Returns: `Output::MaybeValue`". However, the actual handler
//! in handlers/kv.rs returns `Output::Maybe(result)` — which wraps
//! `Option<Value>`, not `Option<VersionedValue>`.
//!
//! This means documentation is misleading: callers who pattern-match on
//! `Output::MaybeVersioned(...)` as shown in the docstring will never match.

use strata_core::value::Value;
use strata_engine::database::Database;
use strata_executor::{BranchId, Command, Executor, Output};

/// Verify that KvGet returns Output::Maybe, not Output::MaybeVersioned as documented.
#[test]
fn issue_954_kvget_returns_maybe_not_maybe_versioned() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Store a value
    executor
        .execute(Command::KvPut {
            branch: Some(branch.clone()),
            key: "k1".into(),
            value: Value::Int(42),
        })
        .unwrap();

    // Retrieve it
    let result = executor
        .execute(Command::KvGet {
            branch: Some(branch.clone()),
            key: "k1".into(),
        })
        .unwrap();

    // The docstring claims this returns MaybeVersioned, but it actually returns Maybe.
    // The handler in kv.rs does: Ok(Output::Maybe(result))
    assert!(
        matches!(result, Output::Maybe(Some(_))),
        "KvGet returns Maybe, not MaybeVersioned as documented. Got: {:?}",
        result
    );

    // Verify it does NOT return MaybeVersioned
    assert!(
        !matches!(result, Output::MaybeVersioned(_)),
        "KvGet should not return MaybeVersioned — the docstring is wrong"
    );
}

/// Verify that KvGet for a missing key returns Maybe(None), not MaybeVersioned(None).
#[test]
fn issue_954_kvget_missing_key_returns_maybe_none() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    let result = executor
        .execute(Command::KvGet {
            branch: Some(branch.clone()),
            key: "nonexistent".into(),
        })
        .unwrap();

    assert!(
        matches!(result, Output::Maybe(None)),
        "KvGet for missing key returns Maybe(None), not MaybeVersioned(None). Got: {:?}",
        result
    );
}
