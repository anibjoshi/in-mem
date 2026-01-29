//! Run module for run lifecycle management and handles
//!
//! This module contains:
//! - `index`: RunIndex for creating, deleting, and managing runs
//! - `handle`: RunHandle facade for run-scoped operations

mod index;
mod handle;

pub use index::{RunIndex, RunMetadata, RunStatus};
pub use handle::{RunHandle, EventHandle, JsonHandle, KvHandle, StateHandle, VectorHandle};
