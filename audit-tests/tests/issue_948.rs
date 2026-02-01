//! Audit test for issue #948: NaN/Infinity in vector embeddings not validated
//! Verdict: CONFIRMED BUG
//!
//! The VectorStore's insert() method validates:
//! - Vector key validity
//! - Collection existence
//! - Dimension match (embedding.len() == config.dimension)
//!
//! But it does NOT validate the actual float values in the embedding. This means:
//! - f32::NAN is accepted silently
//! - f32::INFINITY is accepted silently
//! - f32::NEG_INFINITY is accepted silently
//!
//! NaN values poison similarity calculations: any comparison involving NaN returns false.
//! For cosine similarity, a NaN in any dimension makes the entire dot product NaN,
//! which means the vector will never match any search query (or worse, produce
//! undefined ordering behavior).
//!
//! Infinity values cause similar problems: the magnitude becomes infinite, making
//! normalized cosine similarity undefined.
//!
//! The fix would be to add a validation step in insert():
//! ```ignore
//! for &v in embedding {
//!     if v.is_nan() || v.is_infinite() {
//!         return Err(VectorError::InvalidEmbedding { reason: "..." });
//!     }
//! }
//! ```

use strata_engine::database::Database;
use strata_executor::{BranchId, Command, DistanceMetric, Executor, Output};

/// Demonstrates that NaN values in vector embeddings are silently accepted.
#[test]
fn issue_948_nan_vector_accepted() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Create a collection
    executor
        .execute(Command::VectorCreateCollection {
            branch: Some(branch.clone()),
            collection: "col1".into(),
            dimension: 3,
            metric: DistanceMetric::Cosine,
        })
        .unwrap();

    // Try to insert a vector with NaN
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "col1".into(),
        key: "nan_vec".into(),
        vector: vec![f32::NAN, 0.0, 0.0],
        metadata: None,
    });

    // BUG: This should be rejected but it's accepted
    assert!(
        result.is_ok(),
        "NaN vector was accepted (bug confirmed). \
         Should be rejected with an InvalidEmbedding error."
    );
}

/// Demonstrates that Infinity values in vector embeddings are silently accepted.
#[test]
fn issue_948_infinity_vector_accepted() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Create a collection
    executor
        .execute(Command::VectorCreateCollection {
            branch: Some(branch.clone()),
            collection: "col1".into(),
            dimension: 3,
            metric: DistanceMetric::Cosine,
        })
        .unwrap();

    // Try Infinity
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "col1".into(),
        key: "inf_vec".into(),
        vector: vec![f32::INFINITY, 0.0, 0.0],
        metadata: None,
    });

    // BUG: This should be rejected but it's accepted
    assert!(
        result.is_ok(),
        "Infinity vector was accepted (bug confirmed). \
         Should be rejected with an InvalidEmbedding error."
    );
}

/// Demonstrates that negative infinity is also silently accepted.
#[test]
fn issue_948_neg_infinity_vector_accepted() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Create a collection
    executor
        .execute(Command::VectorCreateCollection {
            branch: Some(branch.clone()),
            collection: "col1".into(),
            dimension: 3,
            metric: DistanceMetric::Cosine,
        })
        .unwrap();

    // Try negative infinity
    let result = executor.execute(Command::VectorUpsert {
        branch: Some(branch.clone()),
        collection: "col1".into(),
        key: "neg_inf_vec".into(),
        vector: vec![f32::NEG_INFINITY, 0.0, 0.0],
        metadata: None,
    });

    // BUG: This should be rejected but it's accepted
    assert!(
        result.is_ok(),
        "Negative infinity vector was accepted (bug confirmed). \
         Should be rejected with an InvalidEmbedding error."
    );
}

/// Demonstrates the downstream impact: NaN vectors poison search results.
#[test]
fn issue_948_nan_vector_poisons_search() {
    let db = Database::ephemeral().unwrap();
    let executor = Executor::new(db);
    let branch = BranchId::from("default");

    // Create a collection
    executor
        .execute(Command::VectorCreateCollection {
            branch: Some(branch.clone()),
            collection: "search_test".into(),
            dimension: 3,
            metric: DistanceMetric::Cosine,
        })
        .unwrap();

    // Insert a valid vector
    executor
        .execute(Command::VectorUpsert {
            branch: Some(branch.clone()),
            collection: "search_test".into(),
            key: "good_vec".into(),
            vector: vec![1.0, 0.0, 0.0],
            metadata: None,
        })
        .unwrap();

    // Insert a NaN vector (should be rejected but isn't)
    executor
        .execute(Command::VectorUpsert {
            branch: Some(branch.clone()),
            collection: "search_test".into(),
            key: "bad_vec".into(),
            vector: vec![f32::NAN, f32::NAN, f32::NAN],
            metadata: None,
        })
        .unwrap();

    // Search for similar vectors
    let search_result = executor
        .execute(Command::VectorSearch {
            branch: Some(branch.clone()),
            collection: "search_test".into(),
            query: vec![1.0, 0.0, 0.0],
            k: 10,
            filter: None,
            metric: None,
        })
        .unwrap();

    match search_result {
        Output::VectorMatches(matches) => {
            // The NaN vector may appear in results with NaN score,
            // or it may be missing depending on how NaN propagates
            // through the similarity calculation.
            for m in &matches {
                if m.key == "bad_vec" {
                    // NaN score: comparison poisoned
                    assert!(
                        m.score.is_nan(),
                        "NaN vector should produce NaN similarity score, \
                         confirming that invalid embeddings poison search results"
                    );
                }
            }
        }
        other => panic!("Expected VectorMatches, got {:?}", other),
    }
}
