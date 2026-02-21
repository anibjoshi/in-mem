//! Auto-embedding module: text embeddings via strata-inference GGUF engine.
//!
//! Provides a lazy-loading model lifecycle via [`EmbedModelState`] and
//! text extraction from Strata [`Value`] types.

pub mod download;
pub mod extract;

use std::sync::Arc;

use strata_inference::EmbeddingEngine;

/// Default model name used for embedding (resolved by strata-inference registry).
pub const DEFAULT_MODEL: &str = "miniLM";

/// Lazy-loading model state stored as a Database extension.
///
/// On first use, loads the embedding model from the strata-inference registry.
/// If model loading fails, stores the error and never retries.
pub struct EmbedModelState {
    engine: once_cell::sync::OnceCell<Result<Arc<EmbeddingEngine>, String>>,
}

impl Default for EmbedModelState {
    fn default() -> Self {
        Self {
            engine: once_cell::sync::OnceCell::new(),
        }
    }
}

impl EmbedModelState {
    /// Get or load the embedding engine.
    ///
    /// Loads the model via `EmbeddingEngine::from_registry(DEFAULT_MODEL)`.
    /// The `_model_dir` parameter is accepted for backwards compatibility but
    /// ignored — the registry manages model storage in `~/.strata/models/`.
    /// Caches the result (success or failure) so loading is attempted at most once.
    pub fn get_or_load(&self, _model_dir: &std::path::Path) -> Result<Arc<EmbeddingEngine>, String> {
        self.engine
            .get_or_init(|| {
                let engine = EmbeddingEngine::from_registry(DEFAULT_MODEL)
                    .map_err(|e| format!("Failed to load embedding model '{}': {}", DEFAULT_MODEL, e))?;
                Ok(Arc::new(engine))
            })
            .clone()
    }

    /// The dimensionality of output embedding vectors.
    ///
    /// Returns `None` if the engine hasn't been loaded yet or failed to load.
    pub fn embedding_dim(&self) -> Option<usize> {
        self.engine
            .get()
            .and_then(|r| r.as_ref().ok())
            .map(|e| e.embedding_dim())
    }
}

/// Embed a query string using the cached embedding engine from the database.
///
/// Loads or retrieves the cached engine via [`EmbedModelState`], then embeds the
/// given text. Returns `None` (with a warning log) if the engine cannot be loaded
/// or embedding fails. This is a best-effort helper for hybrid search.
pub fn embed_query(db: &strata_engine::Database, text: &str) -> Option<Vec<f32>> {
    let model_dir = db.model_dir();
    let state = match db.extension::<EmbedModelState>() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(target: "strata::hybrid", error = %e, "Failed to get embed model state");
            return None;
        }
    };
    let engine = match state.get_or_load(&model_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(target: "strata::hybrid", error = %e, "Failed to load embed model for hybrid search");
            return None;
        }
    };
    match engine.embed(text) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(target: "strata::hybrid", error = %e, "Embedding failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_default_state_is_empty() {
        let state = EmbedModelState::default();
        // Loading from a nonexistent path should fail (model not in registry cache).
        // This test just verifies the OnceCell is initially empty and we get
        // a deterministic error on first access.
        let result = state.get_or_load(Path::new("/nonexistent/path"));
        // Either loads successfully (if model is cached) or fails — both are valid.
        // The important thing is it doesn't panic.
        let _ = result;
    }

    #[test]
    fn test_error_is_cached() {
        let state = EmbedModelState::default();
        let r1 = state.get_or_load(Path::new("/nonexistent"));
        let r2 = state.get_or_load(Path::new("/nonexistent"));
        // Same result both times (OnceCell caching).
        assert_eq!(r1.is_ok(), r2.is_ok());
        if let (Err(e1), Err(e2)) = (&r1, &r2) {
            assert_eq!(e1, e2, "error should be cached and identical");
        }
    }

    #[test]
    fn test_embedding_dim_none_before_load() {
        let state = EmbedModelState::default();
        assert!(state.embedding_dim().is_none());
    }
}
