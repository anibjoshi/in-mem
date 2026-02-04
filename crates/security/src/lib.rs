//! Access control and configuration for Strata database.
//!
//! This crate provides the [`AccessMode`] and [`OpenOptions`] types used to
//! control how a database is opened and what operations are permitted.

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// Controls whether the database allows writes or is read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AccessMode {
    /// Allow both reads and writes (default).
    #[default]
    ReadWrite,
    /// Read-only mode — all write operations return an error.
    ReadOnly,
}

/// Options for opening a database.
///
/// Use the builder pattern to configure options:
///
/// ```ignore
/// use strata_security::{OpenOptions, AccessMode};
///
/// let opts = OpenOptions::new().access_mode(AccessMode::ReadOnly);
/// ```
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// The access mode for the database.
    pub access_mode: AccessMode,
}

impl OpenOptions {
    /// Create a new `OpenOptions` with default settings (read-write mode).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the access mode for the database.
    pub fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = mode;
        self
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            access_mode: AccessMode::ReadWrite,
        }
    }
}
