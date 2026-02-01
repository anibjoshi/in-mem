//! Audit test for issue #939: Branch writes bypass session transaction
//! Verdict: CONFIRMED BUG
//!
//! Session routes BranchCreate, BranchGet, BranchList, BranchExists, and
//! BranchDelete through the executor directly, bypassing any active
//! transaction.
//!
//! From session.rs:82-86:
//! ```ignore
//! Command::BranchCreate { .. }
//! | Command::BranchGet { .. }
//! | Command::BranchList { .. }
//! | Command::BranchExists { .. }
//! | Command::BranchDelete { .. } => self.executor.execute(cmd),
//! ```
//!
//! This means branch creation/deletion during a transaction is immediately
//! committed and is NOT rolled back on TxnRollback. A user who creates a
//! branch inside a transaction and then rolls back will find the branch
//! still exists.

use strata_engine::database::Database;
use strata_executor::{BranchId, Command, Output, Session};

/// Demonstrates that BranchCreate inside a transaction persists after rollback.
#[test]
fn issue_939_branch_create_bypasses_transaction() {
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

    // Create a new branch inside the transaction
    let create_result = session
        .execute(Command::BranchCreate {
            branch_id: Some("test-branch-939".into()),
            metadata: None,
        })
        .unwrap();

    assert!(
        matches!(create_result, Output::BranchWithVersion { .. }),
        "BranchCreate should succeed"
    );

    // Verify branch exists (still inside transaction)
    let exists_result = session
        .execute(Command::BranchExists {
            branch: BranchId::from("test-branch-939"),
        })
        .unwrap();
    assert!(
        matches!(exists_result, Output::Bool(true)),
        "Branch should exist during transaction"
    );

    // Rollback the transaction
    session.execute(Command::TxnRollback).unwrap();

    // BUG: Branch still exists after rollback because BranchCreate
    // bypassed the transaction entirely
    let exists_after = session
        .execute(Command::BranchExists {
            branch: BranchId::from("test-branch-939"),
        })
        .unwrap();

    match exists_after {
        Output::Bool(true) => {
            // BUG CONFIRMED: Branch persists after rollback
        }
        Output::Bool(false) => {
            // If this happens, the bug is fixed
            panic!("Branch was correctly rolled back - bug may be fixed");
        }
        other => panic!("Expected Bool, got: {:?}", other),
    }
}

/// Demonstrates that BranchDelete inside a transaction is also permanent.
#[test]
fn issue_939_branch_delete_bypasses_transaction() {
    let db = Database::ephemeral().unwrap();
    let mut session = Session::new(db);
    let branch = BranchId::from("default");

    // Create a branch outside any transaction
    session
        .execute(Command::BranchCreate {
            branch_id: Some("to-delete-939".into()),
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

    // Delete the branch inside the transaction — bypasses txn
    session
        .execute(Command::BranchDelete {
            branch: BranchId::from("to-delete-939"),
        })
        .unwrap();

    // Rollback the transaction
    session.execute(Command::TxnRollback).unwrap();

    // BUG: Branch is STILL deleted despite rollback
    let exists_after = session
        .execute(Command::BranchExists {
            branch: BranchId::from("to-delete-939"),
        })
        .unwrap();

    match exists_after {
        Output::Bool(false) => {
            // BUG CONFIRMED: Delete was not rolled back
        }
        Output::Bool(true) => {
            panic!("Branch delete was correctly rolled back - bug may be fixed");
        }
        other => panic!("Expected Bool, got: {:?}", other),
    }
}
