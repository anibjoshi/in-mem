//! Public types for the Strata unified API.
//!
//! This module re-exports types from internal crates with a clean public interface.

// ============================================================================
// Public API types - these are what users should use
// ============================================================================

// Core value types
pub use strata_core::Value;

// Version and versioned wrapper
pub use strata_core::Version;
pub use strata_core::Versioned;
pub use strata_core::Timestamp;

// Run types
pub use strata_core::RunId;

// Vector types
pub use strata_core::DistanceMetric;

// Run state and info (users need these for run management)
pub use strata_api::substrate::{RunInfo, RunState, RetentionPolicy};

// Vector search types
pub use strata_api::substrate::{VectorMatch, VectorData, SearchFilter};

// Durability mode for builder configuration
pub use strata_engine::DurabilityMode;

// ============================================================================
// Internal types - not exposed in public API
// ============================================================================

// Internal API types used by primitives
pub(crate) use strata_api::substrate::ApiRunId;

/// Convert a RunId to ApiRunId (internal use only).
pub(crate) fn run_id_to_api(run_id: &RunId) -> ApiRunId {
    let uuid = uuid::Uuid::from_bytes(*run_id.as_bytes());
    ApiRunId::from_uuid(uuid)
}
