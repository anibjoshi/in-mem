//! Integration tests for the embedding engine lifecycle.
//!
//! All tests require real model files and are `#[ignore]` by default.
//! Run with: cargo test -p strata-intelligence --features embed -- --include-ignored

#![cfg(feature = "embed")]

use strata_intelligence::embed::EmbedModelState;
use strata_intelligence::EmbeddingEngine;

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[test]
#[ignore]
fn test_engine_load_and_embed() {
    let engine =
        EmbeddingEngine::from_registry("miniLM").expect("failed to load miniLM from registry");
    let embedding = engine.embed("hello world").expect("embed failed");
    assert_eq!(embedding.len(), 384);
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-4,
        "L2 norm = {}, expected 1.0",
        norm
    );
}

#[test]
#[ignore]
fn test_similar_texts_have_similar_embeddings() {
    let engine =
        EmbeddingEngine::from_registry("miniLM").expect("failed to load miniLM from registry");
    let a = engine
        .embed("the cat sat on the mat")
        .expect("embed failed");
    let b = engine
        .embed("a cat rested on the mat")
        .expect("embed failed");
    let sim = cosine_similarity(&a, &b);
    assert!(
        sim > 0.8,
        "similar texts should have cosine similarity > 0.8, got {}",
        sim
    );
}

#[test]
#[ignore]
fn test_dissimilar_texts_have_low_similarity() {
    let engine =
        EmbeddingEngine::from_registry("miniLM").expect("failed to load miniLM from registry");
    let a = engine.embed("quantum physics").expect("embed failed");
    let b = engine
        .embed("chocolate cake recipe")
        .expect("embed failed");
    let sim = cosine_similarity(&a, &b);
    assert!(
        sim < 0.5,
        "dissimilar texts should have cosine similarity < 0.5, got {}",
        sim
    );
}

#[test]
#[ignore]
fn test_embed_model_state_caches_across_calls() {
    let state = EmbedModelState::default();
    let dir = std::path::Path::new("/unused");

    let arc1 = state.get_or_load(dir).expect("first load");
    let arc2 = state.get_or_load(dir).expect("second load");

    // Same Arc (pointer equality) — engine was only loaded once.
    assert!(
        std::sync::Arc::ptr_eq(&arc1, &arc2),
        "get_or_load should return the same Arc on second call"
    );
}

#[test]
#[ignore]
fn test_embedding_dim_accessor() {
    let engine =
        EmbeddingEngine::from_registry("miniLM").expect("failed to load miniLM from registry");
    assert_eq!(engine.embedding_dim(), 384);
}
