#!/bin/bash
# User Stories for Epic #1: Workspace & Core Types

REPO="anibjoshi/in-mem"
GH="/opt/homebrew/bin/gh"
EPIC_NUMBER=1

# Story 1: Setup Cargo Workspace
$GH issue create \
  --repo $REPO \
  --title "Setup Cargo workspace with crate structure" \
  --milestone "M1: Foundation" \
  --label "user-story,milestone-1,priority-high" \
  --body "## User Story
**As a** developer
**I want** a properly structured Cargo workspace
**So that** I can organize code into logical crates with clear dependencies

## Context
This is the foundation of the entire project. The workspace structure defined here will remain relatively stable throughout the project lifecycle.

## Acceptance Criteria
- [ ] Root \`Cargo.toml\` defines workspace with all member crates
- [ ] Crate structure matches architecture plan:
  - [ ] \`crates/core\` - Core types and traits
  - [ ] \`crates/storage\` - Storage layer
  - [ ] \`crates/concurrency\` - Transactions and OCC
  - [ ] \`crates/durability\` - WAL and snapshots
  - [ ] \`crates/primitives\` - The five primitives
  - [ ] \`crates/engine\` - Main orchestration
  - [ ] \`crates/api\` - Public API
- [ ] Each crate has its own \`Cargo.toml\` with appropriate dependencies
- [ ] \`cargo build\` succeeds for the workspace
- [ ] Each crate has a \`lib.rs\` with basic module structure
- [ ] README.md exists with project overview

## Implementation Notes

### Workspace Cargo.toml
\`\`\`toml
[workspace]
members = [
    \"crates/core\",
    \"crates/storage\",
    \"crates/concurrency\",
    \"crates/durability\",
    \"crates/primitives\",
    \"crates/engine\",
    \"crates/api\",
]
resolver = \"2\"

[workspace.package]
version = \"0.1.0\"
edition = \"2021\"
authors = [\"Your Name <your.email@example.com>\"]
license = \"MIT OR Apache-2.0\"

[workspace.dependencies]
# Shared dependencies across crates
serde = { version = \"1.0\", features = [\"derive\"] }
uuid = { version = \"1.0\", features = [\"v4\", \"serde\"] }
\`\`\`

### Dependency Guidelines
- **core**: No internal dependencies (foundation)
- **storage**: Depends on core
- **concurrency**: Depends on core, storage
- **durability**: Depends on core
- **primitives**: Depends on core, engine
- **engine**: Depends on core, storage, concurrency, durability
- **api**: Depends on engine, primitives

## Testing
- [ ] \`cargo build --workspace\` succeeds
- [ ] \`cargo test --workspace\` runs (even if no tests yet)
- [ ] No circular dependencies

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
2-3 hours
"

# Story 2: Define RunId and Namespace
$GH issue create \
  --repo $REPO \
  --title "Define RunId and Namespace types" \
  --milestone "M1: Foundation" \
  --label "user-story,milestone-1,priority-high" \
  --body "## User Story
**As a** developer
**I want** strongly-typed RunId and Namespace structures
**So that** runs are uniquely identified and properly isolated

## Context
RunId is the fundamental unit of execution in this system. Every write, event, and trace is tagged with a RunId. Namespace provides multi-tenancy and isolation.

## Acceptance Criteria
- [ ] \`RunId\` is a newtype wrapper around \`Uuid\`
- [ ] \`RunId::new()\` generates a new random UUID
- [ ] \`RunId\` implements \`Copy\`, \`Clone\`, \`Debug\`, \`Eq\`, \`Hash\`, \`Serialize\`, \`Deserialize\`
- [ ] \`Namespace\` struct has four fields: tenant, app, agent, run
- [ ] \`Namespace\` provides constructors and accessors
- [ ] Both types are in \`crates/core/src/types.rs\`
- [ ] Documentation explains what runs are (link to AGENT_MENTAL_MODEL.md)

## Implementation

### crates/core/src/types.rs
\`\`\`rust
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a run (bounded execution of an agent)
///
/// A run is a label for grouping writes and events during a time-bounded execution.
/// See AGENT_MENTAL_MODEL.md for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Generate a new random RunId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create RunId from existing UUID (for testing/recovery)
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, \"RunId({})\", self.0)
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

/// Namespace for multi-tenancy and isolation
///
/// Provides hierarchical isolation: tenant > app > agent > run
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Namespace {
    pub tenant: String,
    pub app: String,
    pub agent: String,
    pub run: RunId,
}

impl Namespace {
    pub fn new(tenant: impl Into<String>, app: impl Into<String>, agent: impl Into<String>, run: RunId) -> Self {
        Self {
            tenant: tenant.into(),
            app: app.into(),
            agent: agent.into(),
            run,
        }
    }
}
\`\`\`

## Testing
- [ ] Unit test: Create RunId, verify it's unique
- [ ] Unit test: RunId serialization round-trip (serde)
- [ ] Unit test: Namespace construction and field access
- [ ] Unit test: Namespace equality and hashing

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
2-3 hours
"

# Story 3: Define Key and TypeTag
$GH issue create \
  --repo $REPO \
  --title "Define Key and TypeTag enums" \
  --milestone "M1: Foundation" \
  --label "user-story,milestone-1,priority-high" \
  --body "## User Story
**As a** developer
**I want** a unified Key structure with TypeTag discrimination
**So that** all primitive types can share the same storage backend

## Context
The unified storage approach uses a single BTreeMap for all data types (KV, Events, StateMachine, Trace, RunMetadata). TypeTag discriminates between them.

## Acceptance Criteria
- [ ] \`TypeTag\` enum has variants: KV, Event, StateMachine, Trace, RunMetadata, Vector
- [ ] \`TypeTag\` implements \`Copy\`, \`Clone\`, \`Debug\`, \`Eq\`, \`Hash\`, \`Ord\`, \`Serialize\`, \`Deserialize\`
- [ ] \`Key\` struct contains: namespace, type_tag, user_key (Vec<u8>)
- [ ] \`Key\` implements \`Ord\` for BTreeMap ordering
- [ ] \`Key\` provides constructors for each primitive type
- [ ] Ordering is: namespace > type_tag > user_key (for efficient range scans)

## Implementation

### crates/core/src/types.rs
\`\`\`rust
/// Type discriminator for unified storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TypeTag {
    KV,
    Event,
    StateMachine,
    Trace,
    RunMetadata,
    Vector, // For milestone 2
}

/// Unified key for all storage types
///
/// Ordering: namespace (tenant > app > agent > run) > type_tag > user_key
/// This enables efficient range scans by run or type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    pub namespace: Namespace,
    pub type_tag: TypeTag,
    pub user_key: Vec<u8>,
}

impl Key {
    /// Create a new key
    pub fn new(namespace: Namespace, type_tag: TypeTag, user_key: Vec<u8>) -> Self {
        Self {
            namespace,
            type_tag,
            user_key,
        }
    }

    /// Create a KV key
    pub fn kv(namespace: Namespace, key: impl AsRef<[u8]>) -> Self {
        Self::new(namespace, TypeTag::KV, key.as_ref().to_vec())
    }

    /// Create an Event key (typically with sequence number)
    pub fn event(namespace: Namespace, seq: u64) -> Self {
        Self::new(namespace, TypeTag::Event, seq.to_be_bytes().to_vec())
    }

    /// Create a StateMachine key
    pub fn state_machine(namespace: Namespace, name: impl AsRef<[u8]>) -> Self {
        Self::new(namespace, TypeTag::StateMachine, name.as_ref().to_vec())
    }

    /// Create a Trace key
    pub fn trace(namespace: Namespace, trace_id: Uuid) -> Self {
        Self::new(namespace, TypeTag::Trace, trace_id.as_bytes().to_vec())
    }

    /// Create a RunMetadata key
    pub fn run_metadata(namespace: Namespace) -> Self {
        Self::new(namespace, TypeTag::RunMetadata, vec![])
    }

    /// Check if key matches a prefix (for range scans)
    pub fn starts_with(&self, prefix: &Key) -> bool {
        self.namespace == prefix.namespace
            && self.type_tag == prefix.type_tag
            && self.user_key.starts_with(&prefix.user_key)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order by: namespace, then type_tag, then user_key
        self.namespace
            .tenant
            .cmp(&other.namespace.tenant)
            .then_with(|| self.namespace.app.cmp(&other.namespace.app))
            .then_with(|| self.namespace.agent.cmp(&other.namespace.agent))
            .then_with(|| self.namespace.run.cmp(&other.namespace.run))
            .then_with(|| self.type_tag.cmp(&other.type_tag))
            .then_with(|| self.user_key.cmp(&other.user_key))
    }
}
\`\`\`

## Testing
- [ ] Unit test: TypeTag ordering (KV < Event < StateMachine < Trace < RunMetadata < Vector)
- [ ] Unit test: Key construction for each primitive type
- [ ] Unit test: Key ordering (verify namespace > type_tag > user_key)
- [ ] Unit test: Key prefix matching (for range scans)
- [ ] Unit test: Serialization round-trip

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
3-4 hours
"

# Story 4: Define Value enum and VersionedValue
$GH issue create \
  --repo $REPO \
  --title "Define Value enum and VersionedValue wrapper" \
  --milestone \"M1: Foundation\" \
  --label \"user-story,milestone-1,priority-high\" \
  --body \"## User Story
**As a** developer
**I want** a tagged union Value enum and VersionedValue wrapper
**So that** different primitive types can be stored with version metadata

## Context
The Value enum allows type-safe storage of different primitive payloads in the unified store. VersionedValue adds version, timestamp, and TTL metadata.

## Acceptance Criteria
- [ ] \`Value\` enum has variants for each primitive type
- [ ] Each variant wraps a specific payload type (defined later in primitives crate)
- [ ] For M1: Use \`serde_json::Value\` as placeholder for payloads
- [ ] \`VersionedValue\` struct wraps Value with: version, timestamp, ttl
- [ ] \`Timestamp\` is a newtype wrapper around u64 (millis since epoch)
- [ ] All types implement required traits for storage

## Implementation

### crates/core/src/value.rs
\`\`\`rust
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Timestamp in milliseconds since UNIX epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect(\"Time went backwards\")
            .as_millis() as u64;
        Self(millis)
    }

    pub fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

/// Tagged union of all value types
///
/// M1: Uses serde_json::Value as placeholder
/// M2+: Replace with specific types from primitives crate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    KV(serde_json::Value),
    Event(serde_json::Value),
    StateMachine(serde_json::Value),
    Trace(serde_json::Value),
    RunMetadata(serde_json::Value),
    Vector(serde_json::Value), // M2
}

impl Value {
    pub fn type_tag(&self) -> crate::types::TypeTag {
        use crate::types::TypeTag;
        match self {
            Value::KV(_) => TypeTag::KV,
            Value::Event(_) => TypeTag::Event,
            Value::StateMachine(_) => TypeTag::StateMachine,
            Value::Trace(_) => TypeTag::Trace,
            Value::RunMetadata(_) => TypeTag::RunMetadata,
            Value::Vector(_) => TypeTag::Vector,
        }
    }
}

/// Value with version and metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedValue {
    pub value: Value,
    pub version: u64,
    pub timestamp: Timestamp,
    pub ttl: Option<Duration>,
}

impl VersionedValue {
    pub fn new(value: Value, version: u64) -> Self {
        Self {
            value,
            version,
            timestamp: Timestamp::now(),
            ttl: None,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn is_expired(&self, now: Timestamp) -> bool {
        if let Some(ttl) = self.ttl {
            let expiry = self.timestamp.as_millis() + ttl.as_millis() as u64;
            now.as_millis() >= expiry
        } else {
            false
        }
    }
}
\`\`\`

## Testing
- [ ] Unit test: Timestamp::now() returns reasonable value
- [ ] Unit test: Value::type_tag() returns correct TypeTag
- [ ] Unit test: VersionedValue::new() sets timestamp
- [ ] Unit test: VersionedValue::is_expired() works correctly
- [ ] Unit test: Serialization round-trip for all types

## Dependencies
- Add \`serde_json\` to workspace dependencies

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
2-3 hours
\"

# Story 5: Define Error types
$GH issue create \
  --repo $REPO \
  --title \"Define error type hierarchy\" \
  --milestone \"M1: Foundation\" \
  --label \"user-story,milestone-1,priority-medium\" \
  --body \"## User Story
**As a** developer
**I want** a well-structured error type hierarchy
**So that** errors can be properly categorized and handled

## Context
Error handling is critical for database reliability. Errors should be specific enough for debugging but simple enough to handle.

## Acceptance Criteria
- [ ] \`Error\` enum covers all error categories
- [ ] Each variant has descriptive context
- [ ] Implements \`std::error::Error\`, \`Debug\`, \`Display\`
- [ ] Uses \`thiserror\` for derive macros
- [ ] Result type alias: \`type Result<T> = std::result::Result<T, Error>\`

## Implementation

### crates/core/src/error.rs
\`\`\`rust
use thiserror::Error;

/// Database error types
#[derive(Error, Debug)]
pub enum Error {
    #[error(\"Key not found: {0}\")]
    KeyNotFound(String),

    #[error(\"Transaction conflict: {0}\")]
    TransactionConflict(String),

    #[error(\"CAS failed: expected version {expected}, got {actual}\")]
    CasFailed { expected: u64, actual: u64 },

    #[error(\"IO error: {0}\")]
    Io(#[from] std::io::Error),

    #[error(\"Serialization error: {0}\")]
    Serialization(String),

    #[error(\"Deserialization error: {0}\")]
    Deserialization(String),

    #[error(\"WAL corruption at offset {offset}: {reason}\")]
    WalCorruption { offset: u64, reason: String },

    #[error(\"Recovery error: {0}\")]
    Recovery(String),

    #[error(\"Run not found: {0}\")]
    RunNotFound(String),

    #[error(\"Invalid state: {0}\")]
    InvalidState(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Check if error is retryable (e.g., transaction conflicts)
    pub fn is_retryable(&self) -> bool {
        matches!(self, Error::TransactionConflict(_) | Error::CasFailed { .. })
    }
}
\`\`\`

## Testing
- [ ] Unit test: Error::is_retryable() works correctly
- [ ] Unit test: Error Display formatting is readable
- [ ] Compile test: All error conversions work (io::Error, etc.)

## Dependencies
- Add \`thiserror\` to workspace dependencies

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
1-2 hours
\"

# Story 6: Define Storage and SnapshotView traits
$GH issue create \
  --repo $REPO \
  --title \"Define Storage and SnapshotView traits\" \
  --milestone \"M1: Foundation\" \
  --label \"user-story,milestone-1,priority-high\" \
  --body \"## User Story
**As a** developer
**I want** trait abstractions for Storage and SnapshotView
**So that** implementations can be swapped without changing upper layers

## Context
Traits define contracts. Storage trait allows replacing BTreeMap with sharded/lock-free versions later. SnapshotView prevents API ossification (ClonedSnapshotView now, LazySnapshotView later).

## Acceptance Criteria
- [ ] \`Storage\` trait defines all storage operations
- [ ] \`SnapshotView\` trait defines snapshot read operations
- [ ] Both are in \`crates/core/src/traits.rs\`
- [ ] Traits use associated types or generics where appropriate
- [ ] Comprehensive documentation with examples
- [ ] Traits are Send + Sync for concurrency

## Implementation

### crates/core/src/traits.rs
\`\`\`rust
use crate::error::Result;
use crate::types::{Key, RunId};
use crate::value::VersionedValue;

/// Storage backend abstraction
///
/// Enables replacing BTreeMap with sharded/lock-free implementations later.
pub trait Storage: Send + Sync {
    /// Get current value for key
    fn get(&self, key: &Key) -> Option<VersionedValue>;

    /// Get value as it existed at or before max_version
    fn get_versioned(&self, key: &Key, max_version: u64) -> Option<VersionedValue>;

    /// Put value, returns assigned version
    fn put(&self, key: Key, value: crate::value::Value, ttl: Option<std::time::Duration>) -> Result<u64>;

    /// Delete key, returns old value if existed
    fn delete(&self, key: &Key) -> Option<VersionedValue>;

    /// Scan keys with matching prefix at or before max_version
    fn scan_prefix(&self, prefix: &Key, max_version: u64) -> Vec<(Key, VersionedValue)>;

    /// Scan all keys for a run at or before max_version
    fn scan_by_run(&self, run_id: RunId, max_version: u64) -> Vec<(Key, VersionedValue)>;

    /// Get current global version
    fn current_version(&self) -> u64;

    /// Find keys that have expired (TTL passed)
    fn find_expired_keys(&self, now: crate::value::Timestamp) -> Vec<Key>;
}

/// Snapshot view abstraction
///
/// Represents a version-bounded view of storage.
/// MVP: ClonedSnapshotView (deep clone)
/// Future: LazySnapshotView (lazy reads with version checks)
pub trait SnapshotView: Send + Sync {
    /// Get value from snapshot
    fn get(&self, key: &Key) -> Option<VersionedValue>;

    /// Scan keys with matching prefix in snapshot
    fn scan_prefix(&self, prefix: &Key) -> Vec<(Key, VersionedValue)>;

    /// Get snapshot version
    fn version(&self) -> u64;
}
\`\`\`

## Testing
- [ ] Compile test: Traits compile and can be used as trait objects
- [ ] Example: Show how to implement Storage trait
- [ ] Example: Show how to implement SnapshotView trait

## Related Epic
#${EPIC_NUMBER} - Workspace & Core Types

## Estimated Effort
2-3 hours
\"

echo \"\"
echo \"✅ Created 6 user stories for Epic #1!\"
echo \"\"
"
