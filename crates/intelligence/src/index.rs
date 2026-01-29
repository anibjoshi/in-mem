//! InvertedIndex — re-exported from strata_engine::search
//!
//! The InvertedIndex has been moved to the engine crate so that primitives
//! can use it on write paths. This module re-exports for backward compatibility.

pub use strata_engine::search::{InvertedIndex, PostingEntry, PostingList};
