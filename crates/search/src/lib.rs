//! Pluggable search orchestration for Strata.

pub mod expand;
pub mod fuser;
pub mod hybrid;
pub mod llm_client;
pub mod rerank;

use std::sync::Arc;
use strata_engine::Database;

pub use fuser::{weighted_rrf_fuse, FusedResult, Fuser, RRFFuser};
pub use hybrid::HybridSearch;

/// Trait for embedding query text into a vector.
/// Injected by the executor from strata-intelligence when the embed feature is active.
pub trait QueryEmbedder: Send + Sync {
    /// Embed the given text, returning None on failure.
    fn embed(&self, text: &str) -> Option<Vec<f32>>;
}

/// Extension trait for Database to provide search functionality.
pub trait DatabaseSearchExt {
    /// Get the hybrid search interface
    fn hybrid(&self) -> HybridSearch;
}

impl DatabaseSearchExt for Arc<Database> {
    fn hybrid(&self) -> HybridSearch {
        HybridSearch::new(Arc::clone(self))
    }
}
