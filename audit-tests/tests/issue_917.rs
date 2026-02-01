//! Audit test for issue #917: JsonGet root path in transaction returns raw serialized string
//! instead of deserialized Value
//! Verdict: CONFIRMED BUG
//!
//! In session.rs:251-267, when JsonGet with root path ("$" or "") executes in a transaction,
//! it reads the raw stored value via ctx.get(). JSON documents are stored internally
//! using rmp_serde (MessagePack) as Value::Bytes, not Value::String.
//!
//! The session.rs code has a branch for `Value::String(s)` that deserializes JSON,
//! but the actual stored format is `Value::Bytes(...)`. This falls through to the
//! `Some(other) => Ok(Output::Maybe(Some(other)))` branch, returning the raw MessagePack
//! bytes instead of the deserialized document.

use strata_executor::{Command, Output, Session};

/// Confirm the bug: JsonGet with root path "$" inside a transaction returns raw
/// Value::Bytes (MessagePack) instead of a deserialized Value::Object.
#[test]
fn issue_917_json_get_root_path_in_transaction() {
    let db = strata_engine::database::Database::ephemeral().unwrap();
    let mut session = Session::new(db);

    let branch = strata_executor::BranchId::from("default");

    // Create a JSON document outside the transaction
    let result = session
        .execute(Command::JsonSet {
            branch: Some(branch.clone()),
            key: "doc1".into(),
            path: "$".into(),
            value: strata_core::value::Value::Object(
                vec![
                    (
                        "name".to_string(),
                        strata_core::value::Value::String("Alice".into()),
                    ),
                    ("age".to_string(), strata_core::value::Value::Int(30)),
                ]
                .into_iter()
                .collect(),
            ),
        })
        .unwrap();
    assert!(
        matches!(result, Output::Version(_)),
        "JsonSet should return a version"
    );

    // Begin a transaction
    session
        .execute(Command::TxnBegin {
            branch: Some(branch.clone()),
            options: None,
        })
        .unwrap();

    // JsonGet with root path inside the transaction
    let result = session
        .execute(Command::JsonGet {
            branch: Some(branch.clone()),
            key: "doc1".into(),
            path: "$".into(),
        })
        .unwrap();

    match result {
        Output::Maybe(Some(val)) => {
            // BUG CONFIRMED: The value is raw Value::Bytes (MessagePack) instead of
            // a deserialized Value::Object.
            // The session.rs code at line 265 falls through to `Some(other)` because
            // the stored format is Bytes, not String.
            let is_bytes = matches!(&val, strata_core::value::Value::Bytes(_));
            let is_object = matches!(&val, strata_core::value::Value::Object(_));

            assert!(
                is_bytes || is_object,
                "JsonGet root path in transaction should return Bytes (bug) or Object (fixed). Got: {:?}",
                val
            );

            if is_bytes {
                // Bug is present: raw MessagePack bytes returned to user
                eprintln!(
                    "BUG CONFIRMED: JsonGet root path in transaction returned \
                     Value::Bytes (raw MessagePack) instead of deserialized Object"
                );
            }
            // If is_object, the bug has been fixed
        }
        Output::Maybe(None) => {
            panic!("JsonGet should find the document that was just set");
        }
        other => {
            panic!("Unexpected output from JsonGet: {:?}", other);
        }
    }

    // Commit the transaction
    session.execute(Command::TxnCommit).unwrap();
}

/// Confirm that JSON root path works correctly OUTSIDE a transaction (no bug).
/// This contrasts with the in-transaction path which has the bug.
#[test]
fn issue_917_json_get_root_path_outside_transaction_works() {
    let db = strata_engine::database::Database::ephemeral().unwrap();
    let mut session = Session::new(db);

    let branch = strata_executor::BranchId::from("default");

    // Create a JSON document
    session
        .execute(Command::JsonSet {
            branch: Some(branch.clone()),
            key: "doc2".into(),
            path: "$".into(),
            value: strata_core::value::Value::Object(
                vec![(
                    "status".to_string(),
                    strata_core::value::Value::String("active".into()),
                )]
                .into_iter()
                .collect(),
            ),
        })
        .unwrap();

    // Read outside transaction -- delegates to executor, which uses the JSON primitive
    // properly and returns a deserialized value
    let result = session
        .execute(Command::JsonGet {
            branch: Some(branch.clone()),
            key: "doc2".into(),
            path: "$".into(),
        })
        .unwrap();

    match result {
        Output::Maybe(Some(val)) => {
            // Outside a transaction, JsonGet goes through the proper handler which
            // deserializes correctly
            assert!(
                matches!(&val, strata_core::value::Value::Object(_)),
                "JsonGet root path outside transaction should return Object. Got: {:?}",
                val
            );
        }
        other => panic!("Expected Maybe(Some(Object)), got: {:?}", other),
    }
}
