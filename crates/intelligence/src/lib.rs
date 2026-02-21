//! Embedding and inference infrastructure for Strata.
//!
//! This crate is a thin adapter bridging `strata-inference` (the GGUF inference
//! engine) with `strata-core`/`strata-engine`. All inference features — embedding,
//! model management — are exposed through the executor and CLI, not through
//! `strata-inference`'s own CLI binaries.

#[cfg(feature = "embed")]
pub mod embed;

// Re-export key strata-inference types so that the executor depends only on
// strata-intelligence, not directly on strata-inference.
#[cfg(feature = "embed")]
pub use strata_inference::EmbeddingEngine;
#[cfg(feature = "embed")]
pub use strata_inference::ModelRegistry;
#[cfg(feature = "embed")]
pub use strata_inference::InferenceError;
