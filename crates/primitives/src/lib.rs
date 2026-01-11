//! Primitives layer for in-mem
//!
//! This crate implements the six high-level primitives:
//! - KV Store: Working memory, scratchpads, tool outputs
//! - Event Log: Immutable append-only events with chaining (M3)
//! - State Machine: CAS-based coordination records (M3)
//! - Trace Store: Structured reasoning traces (M3)
//! - Run Index: First-class run metadata with relationships
//! - Vector Store: Semantic search with HNSW (M6)
//!
//! All primitives are stateless facades over the Database engine.

#![warn(missing_docs)]
#![warn(clippy::all)]

// Module declarations
pub mod kv;
// Future primitives (will be implemented across milestones):
// pub mod event_log;       // M3
// pub mod state_machine;   // M3
// pub mod trace;           // M3
// pub mod run_index;       // M3
// pub mod vector;          // M6

// Re-exports
pub use kv::KVStore;
