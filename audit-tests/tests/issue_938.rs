//! Audit test for issue #938: Vector writes bypass session transaction
//! Verdict: CONFIRMED BUG
//!
//! Session routes VectorUpsert, VectorGet, VectorDelete, VectorSearch,
//! VectorCreateCollection, VectorDeleteCollection, and VectorListCollections
//! through the executor directly, bypassing any active transaction.
//!
//! From session.rs:87-93:
//! ```ignore
//! Command::VectorUpsert { .. }
//! | Command::VectorGet { .. }
//! | Command::VectorDelete { .. }
//! | Command::VectorSearch { .. }
//! | Command::VectorCreateCollection { .. }
//! | Command::VectorDeleteCollection { .. }
//! | Command::VectorListCollections { .. } => self.executor.execute(cmd),
//! ```
//!
//! This means vector writes during a transaction are immediately committed
//! (not buffered in the transaction) and are NOT rolled back on TxnRollback.

use strata_engine::database::Database;
use strata_executor::{BranchId, Command, Output, Session};

/// Demonstrates that vector upsert inside a transaction bypasses the
/// transaction and persists even after rollback.
#[test]
fn issue_938_vector_writes_bypass_transaction() {
    let db = Database::ephemeral().unwrap();
    let mut session = Session::new(db);
    let branch = BranchId::from("default");

    // Begin transaction
    session
        .execute(Command::TxnBegin {
            branch: Some(branch.clone()),
            options: None,
        })
        .unwrap();

    // Vector upsert goes through even in transaction (bypasses txn)
    let result = session.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "test_col".into(),
        key: "v1".into(),
        vector: vec![1.0, 0.0, 0.0],
        metadata: None,
    });
    // This succeeds because it bypasses the transaction entirely
    assert!(
        result.is_ok(),
        "Vector upsert should succeed (it bypasses txn)"
    );

    // Rollback transaction
    session.execute(Command::TxnRollback).unwrap();

    // Vector data persists despite rollback — BUG
    let get_result = session
        .execute(Command::VectorGet {
            branch: Some(branch.clone()),
            collection: "test_col".into(),
            key: "v1".into(),
        })
        .unwrap();

    // The vector is still there even though we rolled back
    match get_result {
        Output::VectorData(Some(data)) => {
            // BUG CONFIRMED: Vector data persists after rollback
            assert_eq!(
                data.data.embedding,
                vec![1.0, 0.0, 0.0],
                "Vector persists after rollback because it bypassed the transaction"
            );
        }
        Output::VectorData(None) => {
            // If this happens, the bug is fixed — vector was correctly rolled back
            panic!("Vector data was correctly rolled back - bug may be fixed");
        }
        other => panic!("Expected VectorData, got: {:?}", other),
    }
}

/// Demonstrates that vector delete also bypasses the transaction.
#[test]
fn issue_938_vector_delete_bypasses_transaction() {
    let db = Database::ephemeral().unwrap();
    let mut session = Session::new(db);
    let branch = BranchId::from("default");

    // Create a vector outside any transaction
    session
        .execute(Command::VectorUpsert {
            branch: Some(branch.clone()),
            collection: "col2".into(),
            key: "v1".into(),
            vector: vec![1.0, 2.0, 3.0],
            metadata: None,
        })
        .unwrap();

    // Begin transaction
    session
        .execute(Command::TxnBegin {
            branch: Some(branch.clone()),
            options: None,
        })
        .unwrap();

    // Delete the vector inside the transaction — this bypasses the txn
    let del_result = session
        .execute(Command::VectorDelete {
            branch: Some(branch.clone()),
            collection: "col2".into(),
            key: "v1".into(),
        })
        .unwrap();
    assert!(
        matches!(del_result, Output::Bool(true)),
        "Vector delete should succeed"
    );

    // Rollback the transaction
    session.execute(Command::TxnRollback).unwrap();

    // The vector is STILL deleted despite rollback — BUG
    let get_result = session
        .execute(Command::VectorGet {
            branch: Some(branch.clone()),
            collection: "col2".into(),
            key: "v1".into(),
        })
        .unwrap();

    match get_result {
        Output::VectorData(None) => {
            // BUG CONFIRMED: Delete was not rolled back
        }
        Output::VectorData(Some(_)) => {
            // If this happens, the bug is fixed
            panic!("Vector delete was correctly rolled back - bug may be fixed");
        }
        other => panic!("Expected VectorData, got: {:?}", other),
    }
}
