//! Integration tests for the extract → embed pipeline.
//!
//! These test cross-module behaviors through the public API.

#![cfg(feature = "embed")]

use strata_core::Value;
use strata_intelligence::embed::extract::extract_text;

use std::collections::HashMap;

#[test]
fn test_extract_returns_none_for_non_embeddable() {
    assert!(extract_text(&Value::Null).is_none());
    assert!(extract_text(&Value::Bytes(vec![1, 2, 3])).is_none());
    assert!(extract_text(&Value::String("".into())).is_none());
    assert!(extract_text(&Value::Array(vec![Value::Null, Value::Null])).is_none());
}

#[test]
fn test_extract_complex_value() {
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::String("Alice".into()));
    map.insert(
        "scores".to_string(),
        Value::Array(vec![Value::Int(10), Value::Int(20)]),
    );
    let nested = Value::Object(map);

    let text = extract_text(&nested).unwrap();
    assert!(text.contains("name: Alice"));
    assert!(text.contains("scores:"));
}

#[test]
#[ignore]
fn test_extract_then_embed_roundtrip() {
    let text = extract_text(&Value::String("hello world".into())).unwrap();
    let engine = strata_intelligence::EmbeddingEngine::from_registry("miniLM")
        .expect("failed to load miniLM");
    let embedding = engine.embed(&text).expect("embed failed");
    assert_eq!(embedding.len(), 384);
    // Should be L2-normalized
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-4,
        "L2 norm = {}, expected 1.0",
        norm
    );
}
