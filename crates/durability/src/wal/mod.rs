//! WAL (Write-Ahead Log) module
//!
//! - `legacy`: Single-file WAL (WALEntry, WAL, DurabilityMode)
//! - `config`: WAL configuration (WalConfig, WalConfigError)
//! - `writer`: Segmented WAL writer (WalWriter)
//! - `reader`: Segmented WAL reader (WalReader)

pub mod legacy;
pub mod config;
pub mod reader;
pub mod writer;

// Re-exports from legacy (canonical types)
pub use legacy::{DurabilityMode, WalCorruptionInfo, WalReadResult, WALEntry, WAL};

// Segmented WAL types
pub use config::{WalConfig, WalConfigError};
pub use reader::{TruncateInfo, WalReader, WalReaderError};
pub use writer::WalWriter;
