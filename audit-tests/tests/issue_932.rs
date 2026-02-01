//! Audit test for issue #932: vector_upsert ignores ALL create_collection errors
//! Verdict: CONFIRMED BUG
//!
//! In handlers/vector.rs:86, the auto-create logic uses:
//! ```ignore
//! let _ = p.vector.create_collection(branch_id, &collection, config);
//! ```
//!
//! This discards ALL errors from `create_collection`, not just the expected
//! `AlreadyExists` error. If the collection creation fails for a different
//! reason (e.g., invalid dimension=0, storage error, branch not found),
//! the error is silently swallowed and the subsequent `insert` call will
//! likely fail with a confusing "collection not found" error instead of
//! the real error.

use strata_engine::database::Database;
use strata_executor::{BranchId, Command, Executor, Output};

/// Demonstrates that vector_upsert auto-creates a collection on first insert.
/// The `let _ =` pattern means creation errors are silently ignored.
#[test]
fn issue_932_vector_upsert_auto_creates_collection() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);

    let branch = BranchId::from("default");

    // Upsert without creating collection first — auto-creation should work
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "auto_created".into(),
        key: "v1".into(),
        vector: vec![1.0, 0.0, 0.0],
        metadata: None,
    });

    assert!(
        result.is_ok(),
        "VectorUpsert should auto-create collection and succeed"
    );

    // Verify the collection was created
    let list_result = executor
        .execute(Command::VectorListCollections {
            branch: Some(branch.clone()),
        })
        .unwrap();

    match list_result {
        Output::VectorCollectionList(collections) => {
            let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
            assert!(
                names.contains(&"auto_created"),
                "Collection should exist after auto-creation, found: {:?}",
                names
            );
        }
        other => panic!("Expected VectorCollectionList, got: {:?}", other),
    }
}

/// Demonstrates that a second upsert to the same collection works because
/// the `let _ =` silently ignores the AlreadyExists error. This is the
/// intended behavior for the happy path, but the `let _ =` also swallows
/// other errors.
#[test]
fn issue_932_repeated_upsert_ignores_already_exists() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);

    let branch = BranchId::from("default");

    // First upsert creates the collection
    executor
        .execute(Command::VectorUpsert {
            branch: Some(branch.clone()),
            collection: "col".into(),
            key: "v1".into(),
            vector: vec![1.0, 2.0, 3.0],
            metadata: None,
        })
        .unwrap();

    // Second upsert: create_collection returns AlreadyExists, which is
    // silently ignored via `let _ =`
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "col".into(),
        key: "v2".into(),
        vector: vec![4.0, 5.0, 6.0],
        metadata: None,
    });

    assert!(
        result.is_ok(),
        "Second upsert should succeed (AlreadyExists error silently ignored)"
    );

    // BUG: The `let _ =` pattern means if create_collection fails for ANY
    // reason (e.g., storage error, invalid config), it is also silently
    // ignored. Only AlreadyExists should be ignored; other errors should
    // propagate.
}

/// Demonstrates that upserting with a different dimension to an existing
/// collection does not fail at collection creation (the error is swallowed).
/// Instead, it may fail at the insert step with a dimension mismatch.
#[test]
fn issue_932_dimension_change_error_swallowed() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);

    let branch = BranchId::from("default");

    // Create collection with dimension 3
    executor
        .execute(Command::VectorUpsert {
            branch: Some(branch.clone()),
            collection: "dim_test".into(),
            key: "v1".into(),
            vector: vec![1.0, 2.0, 3.0],
            metadata: None,
        })
        .unwrap();

    // Try to upsert with dimension 5 — create_collection will attempt
    // to create with dim=5, get AlreadyExists, and the error is silently
    // swallowed. Then insert will fail with dimension mismatch.
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "dim_test".into(),
        key: "v2".into(),
        vector: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        metadata: None,
    });

    // The dimension mismatch error comes from the insert step, not from
    // the swallowed create_collection error. This is confusing but works
    // in practice because AlreadyExists is the only non-error case.
    assert!(
        result.is_err(),
        "Should fail with dimension mismatch on insert"
    );
}
