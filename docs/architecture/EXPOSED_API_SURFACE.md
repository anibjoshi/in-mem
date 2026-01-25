# Strata Exposed API Surface

> **Status**: Audit Document for Review
> **Author**: Architecture Team
> **Date**: 2026-01-25

---

## Executive Summary

This document catalogs all APIs we plan to expose to users and identifies areas that need cleanup. The API follows a two-layer model:

1. **Substrate API**: Power-user surface with explicit runs, versions, and transactions
2. **Facade API**: Redis-like convenience surface with implicit defaults

**Key Issues Identified:**
- Inconsistent naming conventions across primitives
- Some internal types leaking into public API
- Method signature inconsistencies
- Missing unified entry point (no root `strata` crate yet)

---

## Table of Contents

1. [Entry Point: Database](#1-entry-point-database)
2. [Substrate API](#2-substrate-api)
3. [Facade API](#3-facade-api)
4. [Core Types](#4-core-types)
5. [Issues & Cleanup Needed](#5-issues--cleanup-needed)
6. [Recommended Final API Surface](#6-recommended-final-api-surface)

---

## 1. Entry Point: Database

### 1.1 DatabaseBuilder (Configuration)

```rust
// Opening a database
let db = Database::builder()
    .path("./my-db")           // Set storage path
    .buffered()                // Use buffered durability (default production)
    .open()?;

// Or in-memory for testing
let db = Database::builder()
    .in_memory()
    .open_temp()?;
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `builder()` | Create DatabaseBuilder | ✅ Yes |
| `path(path)` | Set database path | ✅ Yes |
| `in_memory()` | Use in-memory storage | ✅ Yes |
| `buffered()` | Buffered durability (default) | ✅ Yes |
| `buffered_with(ms, count)` | Custom buffered settings | ⚠️ Advanced |
| `strict()` | Strict fsync mode | ⚠️ Advanced |
| `durability(mode)` | Set durability mode directly | ⚠️ Advanced |
| `open()` | Open database | ✅ Yes |
| `open_temp()` | Open temp database | ✅ Yes (testing) |

### 1.2 Database (Lifecycle)

| Method | Description | Expose? |
|--------|-------------|---------|
| `storage()` | Access storage layer | ❌ **Internal** |
| `wal()` | Access WAL | ❌ **Internal** |
| `flush()` | Force flush to disk | ⚠️ Advanced |
| `shutdown()` | Graceful shutdown | ✅ Yes |
| `is_open()` | Check if open | ✅ Yes |
| `close()` | Close database | ✅ Yes |
| `coordinator()` | Transaction coordinator | ❌ **Internal** |
| `metrics()` | Transaction metrics | ⚠️ Advanced |
| `durability_mode()` | Get durability mode | ✅ Yes |

### 1.3 Database (Transaction API)

| Method | Description | Expose? |
|--------|-------------|---------|
| `transaction(run_id, closure)` | Execute in transaction | ✅ Yes |
| `transaction_with_retry(...)` | Transaction with retry | ⚠️ Advanced |
| `transaction_with_timeout(...)` | Transaction with timeout | ⚠️ Advanced |
| `transaction_with_durability(...)` | Override durability | ❌ **Internal** |
| `begin_transaction(run_id)` | Manual begin | ❌ **Internal** |
| `end_transaction(ctx)` | Manual end | ❌ **Internal** |
| `commit_transaction(ctx)` | Manual commit | ❌ **Internal** |

### 1.4 Database (Low-level - SHOULD NOT EXPOSE)

| Method | Description | Expose? |
|--------|-------------|---------|
| `put(run_id, key, value)` | Direct put | ❌ **Internal** |
| `get(key)` | Direct get | ❌ **Internal** |
| `delete(run_id, key)` | Direct delete | ❌ **Internal** |
| `cas(...)` | Direct CAS | ❌ **Internal** |
| `replay_run(run_id)` | Replay run | ⚠️ Advanced |
| `diff_runs(...)` | Diff two runs | ⚠️ Advanced |
| `extension<T>()` | Type-erased extension | ❌ **Internal** |

### 1.5 RetryConfig

```rust
let config = RetryConfig::new()
    .with_max_retries(3)
    .with_base_delay_ms(10)
    .with_max_delay_ms(1000);
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `new()` | Default config | ✅ Yes |
| `no_retry()` | Disable retry | ✅ Yes |
| `with_max_retries(n)` | Set max retries | ✅ Yes |
| `with_base_delay_ms(ms)` | Set base delay | ✅ Yes |
| `with_max_delay_ms(ms)` | Set max delay | ✅ Yes |

---

## 2. Substrate API

The Substrate API provides explicit control over all database operations.

### 2.1 Core Types

#### ApiRunId

```rust
let run = ApiRunId::new();              // Random UUID
let run = ApiRunId::default();          // "default" run
let run = ApiRunId::parse("my-run")?;   // Parse from string
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `new()` | Create random UUID run | ✅ Yes |
| `default()` / `default_run_id()` | Get default run | ✅ Yes |
| `parse(&str)` | Parse from string | ✅ Yes |
| `is_default()` | Check if default | ✅ Yes |
| `is_uuid()` | Check if UUID | ✅ Yes |
| `as_str()` | Get as string | ✅ Yes |
| `into_string()` | Convert to String | ✅ Yes |
| `from_uuid(uuid)` | Create from UUID | ⚠️ Advanced |
| `as_uuid()` | Get as UUID | ⚠️ Advanced |
| `to_run_id()` | Convert to internal RunId | ❌ **Internal** |

#### RunState

```rust
pub enum RunState {
    Active,     // Run is active
    Completed,  // Run completed successfully
    Failed,     // Run failed
    Cancelled,  // Run was cancelled
    Paused,     // Run is paused
    Archived,   // Run is archived (terminal)
}
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `is_active()` | Check if active | ✅ Yes |
| `is_closed()` | Check if closed | ✅ Yes |
| `is_terminal()` | Check if terminal state | ✅ Yes |
| `is_finished()` | Check if finished | ✅ Yes |
| `is_paused()` | Check if paused | ✅ Yes |
| `as_str()` | Get as string | ✅ Yes |

#### RunInfo

```rust
pub struct RunInfo {
    pub run_id: ApiRunId,
    pub created_at: Timestamp,
    pub metadata: Value,
    pub state: RunState,
    pub error: Option<String>,
}
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `new(run_id, metadata)` | Create RunInfo | ✅ Yes |
| `default_run()` | Get default run info | ✅ Yes |
| `is_default()` | Check if default | ✅ Yes |
| `is_active()` | Check if active | ✅ Yes |
| `is_closed()` | Check if closed | ✅ Yes |
| `close()` | Mark as completed | ❌ **Internal** (use RunIndex) |
| `fail(error)` | Mark as failed | ❌ **Internal** (use RunIndex) |

#### RetentionPolicy

```rust
pub enum RetentionPolicy {
    KeepAll,                    // Keep all versions
    KeepLast(u64),              // Keep last N versions
    KeepFor(Duration),          // Keep for duration
    Composite(Vec<Self>),       // Multiple policies
}
```

| Method | Description | Expose? |
|--------|-------------|---------|
| `keep_last(n)` | Keep last N | ✅ Yes |
| `keep_for(duration)` | Keep for duration | ✅ Yes |
| `composite(policies)` | Combine policies | ✅ Yes |
| `is_keep_all()` | Check if keep all | ✅ Yes |
| `should_keep_by_count(...)` | Evaluate count policy | ❌ **Internal** |
| `should_keep_by_age(...)` | Evaluate age policy | ❌ **Internal** |

### 2.2 KVStore Trait

```rust
pub trait KVStore {
    // Core CRUD
    fn kv_put(&self, run: &ApiRunId, key: &str, value: Value) -> Result<Version>;
    fn kv_get(&self, run: &ApiRunId, key: &str) -> Result<Option<Versioned<Value>>>;
    fn kv_delete(&self, run: &ApiRunId, key: &str) -> Result<bool>;
    fn kv_exists(&self, run: &ApiRunId, key: &str) -> Result<bool>;

    // Atomic operations
    fn kv_incr(&self, run: &ApiRunId, key: &str, delta: i64) -> Result<i64>;
    fn kv_cas_version(&self, run: &ApiRunId, key: &str, expected: Version, value: Value) -> Result<bool>;
    fn kv_cas_value(&self, run: &ApiRunId, key: &str, expected: Value, value: Value) -> Result<bool>;

    // History & scanning
    fn kv_get_at(&self, run: &ApiRunId, key: &str, version: Version) -> Result<Versioned<Value>>;
    fn kv_history(&self, run: &ApiRunId, key: &str, limit: usize, before: Option<Version>) -> Result<Vec<Versioned<Value>>>;
    fn kv_keys(&self, run: &ApiRunId, prefix: &str, limit: usize) -> Result<Vec<String>>;
    fn kv_scan(&self, run: &ApiRunId, prefix: &str, limit: usize, cursor: Option<String>) -> Result<KVScanResult>;
}

pub trait KVStoreBatch: KVStore {
    fn kv_mget(&self, run: &ApiRunId, keys: &[&str]) -> Result<Vec<Option<Versioned<Value>>>>;
    fn kv_mput(&self, run: &ApiRunId, entries: &[(&str, Value)]) -> Result<Version>;
    fn kv_mdelete(&self, run: &ApiRunId, keys: &[&str]) -> Result<u64>;
    fn kv_mexists(&self, run: &ApiRunId, keys: &[&str]) -> Result<u64>;
}
```

**Issues:**
- ⚠️ `kv_` prefix on all methods is verbose
- ⚠️ Should `KVStoreBatch` be merged into `KVStore`?

### 2.3 JsonStore Trait

```rust
pub trait JsonStore {
    // Core operations
    fn json_set(&self, run: &ApiRunId, key: &str, path: &str, value: Value) -> Result<Version>;
    fn json_get(&self, run: &ApiRunId, key: &str, path: &str) -> Result<Option<Versioned<Value>>>;
    fn json_delete(&self, run: &ApiRunId, key: &str, path: &str) -> Result<u64>;
    fn json_merge(&self, run: &ApiRunId, key: &str, path: &str, patch: Value) -> Result<Version>;
    fn json_exists(&self, run: &ApiRunId, key: &str) -> Result<bool>;

    // Array operations
    fn json_array_push(&self, run: &ApiRunId, key: &str, path: &str, values: Vec<Value>) -> Result<usize>;
    fn json_array_pop(&self, run: &ApiRunId, key: &str, path: &str) -> Result<Option<Value>>;
    fn json_increment(&self, run: &ApiRunId, key: &str, path: &str, delta: f64) -> Result<f64>;

    // Querying
    fn json_query(&self, run: &ApiRunId, path: &str, value: Value, limit: usize) -> Result<Vec<String>>;
    fn json_search(&self, run: &ApiRunId, query: &str, k: usize) -> Result<Vec<JsonSearchHit>>;
    fn json_list(&self, run: &ApiRunId, prefix: &str, cursor: Option<String>, limit: usize) -> Result<JsonListResult>;

    // Batch & CAS
    fn json_cas(&self, run: &ApiRunId, key: &str, expected: Version, path: &str, value: Value) -> Result<Version>;
    fn json_batch_get(&self, run: &ApiRunId, keys: &[&str]) -> Result<Vec<Option<Versioned<Value>>>>;
    fn json_batch_create(&self, run: &ApiRunId, docs: Vec<Value>) -> Result<Vec<Version>>;

    // History
    fn json_history(&self, run: &ApiRunId, key: &str, limit: usize, before: Option<Version>) -> Result<Vec<Versioned<Value>>>;
    fn json_get_version(&self, run: &ApiRunId, key: &str) -> Result<Option<u64>>;
    fn json_count(&self, run: &ApiRunId) -> Result<u64>;
}
```

**Issues:**
- ⚠️ `json_search` vs `json_query` naming unclear
- ⚠️ Path parameter inconsistency (`path: &str` vs JSONPath type)

### 2.4 EventLog Trait

```rust
pub trait EventLog {
    // Append operations
    fn event_append(&self, run: &ApiRunId, stream: &str, payload: Value) -> Result<Version>;
    fn event_append_batch(&self, run: &ApiRunId, events: Vec<(&str, Value)>) -> Result<Vec<Version>>;

    // Read operations
    fn event_get(&self, run: &ApiRunId, stream: &str, sequence: u64) -> Result<Option<Versioned<Value>>>;
    fn event_range(&self, run: &ApiRunId, stream: &str, start: u64, end: u64, limit: usize) -> Result<Vec<Versioned<Value>>>;
    fn event_len(&self, run: &ApiRunId, stream: &str) -> Result<u64>;
    fn event_latest_sequence(&self, run: &ApiRunId, stream: &str) -> Result<Option<u64>>;
}
```

**Issues:**
- ⚠️ Uses `Version` return but EventLog uses sequence numbers internally
- ⚠️ Missing stream listing operations

### 2.5 StateCell Trait

```rust
pub trait StateCell {
    fn state_set(&self, run: &ApiRunId, cell: &str, value: Value) -> Result<Version>;
    fn state_get(&self, run: &ApiRunId, cell: &str) -> Result<Option<Versioned<Value>>>;
    fn state_cas(&self, run: &ApiRunId, cell: &str, expected: u64, value: Value) -> Result<Option<Version>>;
    fn state_delete(&self, run: &ApiRunId, cell: &str) -> Result<bool>;
    fn state_exists(&self, run: &ApiRunId, cell: &str) -> Result<bool>;
    fn state_history(&self, run: &ApiRunId, cell: &str, limit: usize, before: Option<Version>) -> Result<Vec<Versioned<Value>>>;
}
```

**Issues:**
- ⚠️ `state_cas` takes `expected: u64` but returns `Option<Version>` - inconsistent
- ⚠️ Should CAS return `Result<bool>` or `Result<Option<Version>>`?

### 2.6 VectorStore Trait

```rust
pub trait VectorStore {
    // CRUD
    fn vector_upsert(&self, run: &ApiRunId, collection: &str, key: &str, vector: Vec<f32>, metadata: Value) -> Result<Version>;
    fn vector_upsert_with_source(&self, run: &ApiRunId, collection: &str, key: &str, vector: Vec<f32>, metadata: Value, source_ref: EntityRef) -> Result<Version>;
    fn vector_get(&self, run: &ApiRunId, collection: &str, key: &str) -> Result<Option<Versioned<VectorData>>>;
    fn vector_delete(&self, run: &ApiRunId, collection: &str, key: &str) -> Result<bool>;
    fn vector_exists(&self, run: &ApiRunId, collection: &str, key: &str) -> Result<bool>;

    // Search
    fn vector_search(&self, run: &ApiRunId, collection: &str, query: Vec<f32>, k: usize, metric: DistanceMetric, filter: Option<SearchFilter>) -> Result<Vec<VectorMatch>>;

    // Collection management
    fn vector_count(&self, run: &ApiRunId, collection: &str) -> Result<u64>;
    fn vector_list_collections(&self, run: &ApiRunId) -> Result<Vec<String>>;
    fn vector_collection_info(&self, run: &ApiRunId, collection: &str) -> Result<VectorCollectionInfo>;
}

pub enum DistanceMetric { Cosine, Euclidean, DotProduct }

pub enum SearchFilter {
    Equals(String, Value),
    Prefix(String, String),
    Range { field: String, min: Option<f64>, max: Option<f64> },
    And(Vec<SearchFilter>),
    Or(Vec<SearchFilter>),
    Not(Box<SearchFilter>),
}
```

**Issues:**
- ⚠️ `VectorData` type vs `VectorMatch` type - redundant?
- ⚠️ `vector_upsert_with_source` seems specialized

### 2.7 RunIndex Trait

```rust
pub trait RunIndex {
    fn run_create(&self, run_id: ApiRunId, metadata: Value) -> Result<(RunInfo, Version)>;
    fn run_get(&self, run: &ApiRunId) -> Result<Option<Versioned<RunInfo>>>;
    fn run_list(&self, state: Option<RunState>, limit: usize, offset: usize) -> Result<Vec<Versioned<RunInfo>>>;
    fn run_exists(&self, run: &ApiRunId) -> Result<bool>;
    fn run_update_metadata(&self, run: &ApiRunId, metadata: Value) -> Result<Version>;

    // State transitions
    fn run_close(&self, run: &ApiRunId) -> Result<Version>;
    fn run_pause(&self, run: &ApiRunId) -> Result<Version>;
    fn run_resume(&self, run: &ApiRunId) -> Result<Version>;
    fn run_fail(&self, run: &ApiRunId, error: String) -> Result<Version>;
    fn run_cancel(&self, run: &ApiRunId) -> Result<Version>;
    fn run_archive(&self, run: &ApiRunId) -> Result<Version>;
}
```

**Issues:**
- ⚠️ `run_create` returns `(RunInfo, Version)` - inconsistent with other methods
- ⚠️ Should use `Versioned<RunInfo>` for consistency?

### 2.8 TransactionControl Trait

```rust
pub trait TransactionControl {
    fn txn_begin(&self, options: TxnOptions) -> Result<TxnId>;
    fn txn_commit(&self) -> Result<Version>;
    fn txn_rollback(&self) -> Result<()>;
    fn txn_info(&self) -> Option<TxnInfo>;
    fn txn_is_active(&self) -> bool;
}

pub trait TransactionSavepoint: TransactionControl {
    fn savepoint(&self, name: &str) -> Result<()>;
    fn rollback_to(&self, name: &str) -> Result<()>;
    fn release_savepoint(&self, name: &str) -> Result<()>;
}

pub struct TxnOptions {
    pub timeout_ms: Option<u64>,
    pub read_locks: bool,
}
```

**Issues:**
- ⚠️ `TransactionControl` exposed but transactions typically managed by Database
- ⚠️ Should this be exposed at all, or hidden behind `Database::transaction()`?

---

## 3. Facade API

The Facade API provides Redis-like simplicity with implicit defaults.

### 3.1 KVFacade Trait

```rust
pub trait KVFacade {
    // Simple operations
    fn get(&self, key: &str) -> Result<Option<Value>>;
    fn getv(&self, key: &str) -> Result<Option<Versioned<Value>>>;  // With version
    fn set(&self, key: &str, value: Value) -> Result<()>;
    fn del(&self, key: &str) -> Result<bool>;
    fn exists(&self, key: &str) -> Result<bool>;

    // Atomic increment
    fn incr(&self, key: &str) -> Result<i64>;
    fn incrby(&self, key: &str, delta: i64) -> Result<i64>;
    fn decrby(&self, key: &str, delta: i64) -> Result<i64>;

    // Conditional operations
    fn setnx(&self, key: &str, value: Value) -> Result<bool>;  // Set if not exists
    fn setex(&self, key: &str, ttl: Duration, value: Value) -> Result<()>;  // Set with TTL
    fn getdel(&self, key: &str) -> Result<Option<Value>>;  // Get and delete
    fn getex(&self, key: &str, options: GetOptions) -> Result<Option<Value>>;
    fn set_with_options(&self, key: &str, value: Value, options: SetOptions) -> Result<Option<Value>>;
    fn get_with_options(&self, key: &str, options: GetOptions) -> Result<Option<(Value, Option<u64>)>>;
    fn incr_with_options(&self, key: &str, delta: i64, options: IncrOptions) -> Result<i64>;
}

pub trait KVFacadeBatch: KVFacade {
    fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Value>>>;
    fn mset(&self, entries: &[(&str, Value)]) -> Result<()>;
    fn del_many(&self, keys: &[&str]) -> Result<u64>;
    fn keys(&self, prefix: &str, limit: usize) -> Result<Vec<String>>;
    fn scan(&self, prefix: &str, limit: usize, cursor: Option<String>) -> Result<KVScanResult>;
}
```

**Issues:**
- ⚠️ `setex` implies TTL but we don't have TTL infrastructure
- ⚠️ Too many options structs (`GetOptions`, `SetOptions`, `IncrOptions`)

### 3.2 JsonFacade Trait

```rust
pub trait JsonFacade {
    fn json_get(&self, key: &str, path: &str) -> Result<Option<Value>>;
    fn json_getv(&self, key: &str, path: &str) -> Result<Option<Versioned<Value>>>;
    fn json_set(&self, key: &str, path: &str, value: Value) -> Result<()>;
    fn json_del(&self, key: &str, path: &str) -> Result<u64>;
    fn json_merge(&self, key: &str, path: &str, patch: Value) -> Result<()>;

    // Type inspection
    fn json_type(&self, key: &str, path: &str) -> Result<Option<String>>;
    fn json_strlen(&self, key: &str, path: &str) -> Result<Option<usize>>;
    fn json_arrlen(&self, key: &str, path: &str) -> Result<Option<usize>>;
    fn json_objkeys(&self, key: &str, path: &str) -> Result<Vec<String>>;
}
```

### 3.3 EventFacade Trait

```rust
pub trait EventFacade {
    fn xadd(&self, stream: &str, payload: Value) -> Result<u64>;  // Returns sequence
    fn xrange(&self, stream: &str, start: u64, end: u64) -> Result<Vec<EventEntry>>;
    fn xrange_count(&self, stream: &str, start: u64, end: u64, count: usize) -> Result<Vec<EventEntry>>;
    fn xlen(&self, stream: &str) -> Result<u64>;
    fn xinfo(&self, stream: &str) -> Result<StreamInfo>;
}

pub struct EventEntry {
    pub sequence: u64,
    pub payload: Value,
    pub timestamp: u64,
}
```

**Issues:**
- ⚠️ Redis-style `x` prefix doesn't match other Facade naming
- ⚠️ Should be `event_add`, `event_range`, etc.?

### 3.4 StateFacade Trait

```rust
pub trait StateFacade {
    fn state_set(&self, cell: &str, value: Value) -> Result<()>;
    fn state_get(&self, cell: &str) -> Result<Option<Versioned<Value>>>;
    fn state_cas(&self, cell: &str, expected: u64, value: Value) -> Result<bool>;
    fn state_delete(&self, cell: &str) -> Result<bool>;
    fn state_exists(&self, cell: &str) -> Result<bool>;
}
```

**Issues:**
- ⚠️ `state_get` returns `Versioned` but other Facade methods strip versions

### 3.5 VectorFacade Trait

```rust
pub trait VectorFacade {
    fn vadd(&self, collection: &str, key: &str, vector: Vec<f32>, metadata: Value) -> Result<()>;
    fn vget(&self, collection: &str, key: &str) -> Result<Option<VectorResult>>;
    fn vdel(&self, collection: &str, key: &str) -> Result<bool>;
    fn vsim(&self, collection: &str, query: Vec<f32>, k: usize) -> Result<Vec<VectorResult>>;
    fn vsim_filter(&self, collection: &str, query: Vec<f32>, k: usize, options: VectorSearchOptions) -> Result<Vec<VectorResult>>;
    fn vcollections(&self) -> Result<Vec<VectorCollectionSummary>>;
    fn vcount(&self, collection: &str) -> Result<u64>;
}
```

**Issues:**
- ⚠️ `v` prefix inconsistent with other facades
- ⚠️ Should be `vector_add`, `vector_search`, etc.?

### 3.6 HistoryFacade Trait

```rust
pub trait HistoryFacade {
    fn kv_history(&self, key: &str, limit: usize) -> Result<Vec<Versioned<Value>>>;
    fn json_history(&self, key: &str, limit: usize) -> Result<Vec<Versioned<Value>>>;
    fn state_history(&self, cell: &str, limit: usize) -> Result<Vec<Versioned<Value>>>;
}
```

### 3.7 RunFacade Trait

```rust
pub trait RunFacade {
    fn run_create(&self, metadata: Value) -> Result<RunInfo>;
    fn run_list(&self, state: Option<RunState>, limit: usize) -> Result<Vec<RunInfo>>;
    fn run_get_current(&self) -> Result<RunInfo>;
}

pub trait ScopedFacade {
    fn scoped_to(&self, run_id: ApiRunId) -> Self;
}
```

### 3.8 SystemFacade Trait

```rust
pub trait SystemFacade {
    fn shutdown(&self) -> Result<()>;
    fn get_capabilities(&self) -> Capabilities;
}

pub trait Capabilities {
    fn get_limits(&self) -> CapabilityLimits;
}

pub struct CapabilityLimits {
    pub max_key_size: usize,
    pub max_value_size: usize,
    pub max_transaction_size: usize,
}
```

---

## 4. Core Types

### 4.1 Value

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

### 4.2 Versioned<T>

```rust
pub struct Versioned<T> {
    value: T,
    version: Version,
    timestamp: Timestamp,
}
```

| Method | Expose? |
|--------|---------|
| `new(value, version)` | ✅ Yes |
| `with_timestamp(value, version, timestamp)` | ✅ Yes |
| `value()` / `into_value()` | ✅ Yes |
| `version()` | ✅ Yes |
| `timestamp()` | ✅ Yes |
| `map(f)` | ✅ Yes |

### 4.3 Version

```rust
pub struct Version { ... }
```

| Method | Expose? |
|--------|---------|
| `txn(id)` | ✅ Yes |
| `seq(n)` | ✅ Yes |
| `counter(n)` | ✅ Yes |
| `as_u64()` | ✅ Yes |
| `is_zero()` | ✅ Yes |
| `is_txn_id()` / `is_sequence()` / `is_counter()` | ⚠️ Advanced |

### 4.4 Timestamp

```rust
pub struct Timestamp { ... }
```

| Method | Expose? |
|--------|---------|
| `now()` | ✅ Yes |
| `from_micros(us)` | ✅ Yes |
| `as_micros()` | ✅ Yes |

### 4.5 Error Types

```rust
pub enum Error {
    IoError(io::Error),
    SerializationError(String),
    KeyNotFound(Key),
    RunNotFound(RunId),
    TransactionAborted(String),
    VersionMismatch { expected: u64, actual: u64 },
    InvalidOperation(String),
    Conflict(String),
    Internal(String),
}

pub struct StrataError {
    code: ErrorCode,
    message: String,
    details: Option<ErrorDetails>,
}

pub enum ErrorCode {
    NotFound,
    WrongType,
    InvalidKey,
    InvalidPath,
    HistoryTrimmed,
    ConstraintViolation,
    Conflict,
    SerializationError,
    StorageError,
    InternalError,
}
```

**Issues:**
- ⚠️ Two error types (`Error` and `StrataError`) - confusing
- ⚠️ Should consolidate to single error type

---

## 5. Issues & Cleanup Needed

### 5.1 Naming Inconsistencies

| Issue | Current | Recommended |
|-------|---------|-------------|
| Facade method prefix | `xadd`, `vadd`, `json_get` | Consistent: `event_add`, `vector_add`, `json_get` |
| Substrate trait naming | `KVStore`, `JsonStore`, `EventLog` | Consistent: `*Store` or `*Log` |
| Batch trait separation | `KVStoreBatch` separate | Merge into main trait |

### 5.2 Type Leakage

| Type | Current | Recommended |
|------|---------|-------------|
| `SubstrateImpl` | `pub use impl_::SubstrateImpl` | Should be internal |
| `FacadeImpl` | `pub use impl_::FacadeImpl` | Should be internal |
| `RunId` (internal) | Exposed via `to_run_id()` | Should not expose |
| `Key` (internal) | Exposed in errors | Wrap in API error |

### 5.3 Return Type Inconsistencies

| Method | Current | Recommended |
|--------|---------|-------------|
| `run_create` | `(RunInfo, Version)` | `Versioned<RunInfo>` |
| `state_get` (facade) | `Versioned<Value>` | `Value` (like other facade methods) |
| `state_cas` | `Option<Version>` | `Result<bool>` |
| `event_append` | `Version` | `u64` (sequence number) |

### 5.4 Missing Unified Entry Point

Currently users must:
```rust
use strata_engine::Database;
use strata_api::substrate::KVStore;
use strata_api::facade::KVFacade;
```

Should be:
```rust
use strata::prelude::*;
// Database, KVStore, KVFacade, etc. all available
```

### 5.5 Options Struct Proliferation

- `GetOptions`, `SetOptions`, `IncrOptions`, `TxnOptions`, `VectorSearchOptions`
- Consider builder pattern or method chaining instead

### 5.6 Missing APIs

| Category | Missing |
|----------|---------|
| EventLog | Stream listing, stream deletion |
| VectorStore | Batch operations |
| All | TTL operations (mentioned but not implemented) |
| All | Watch/subscription APIs |

---

## 6. Recommended Final API Surface

### 6.1 Entry Point Hierarchy

```rust
// User imports
use strata::prelude::*;

// Entry point
let db = Strata::builder()
    .path("./my-db")
    .open()?;

// Get API surfaces
let substrate = db.substrate();  // Returns impl KVStore + JsonStore + ...
let facade = db.facade();        // Returns impl KVFacade + JsonFacade + ...
```

### 6.2 Cleaned Substrate Traits

```rust
// Unified primitive access
pub trait KVStore {
    fn put(&self, run: &RunId, key: &str, value: Value) -> Result<Version>;
    fn get(&self, run: &RunId, key: &str) -> Result<Option<Versioned<Value>>>;
    fn delete(&self, run: &RunId, key: &str) -> Result<bool>;
    fn exists(&self, run: &RunId, key: &str) -> Result<bool>;
    fn incr(&self, run: &RunId, key: &str, delta: i64) -> Result<i64>;
    fn cas(&self, run: &RunId, key: &str, expected: Version, value: Value) -> Result<bool>;
    fn history(&self, run: &RunId, key: &str, limit: usize) -> Result<Vec<Versioned<Value>>>;
    fn keys(&self, run: &RunId, prefix: &str, limit: usize) -> Result<Vec<String>>;
    fn scan(&self, run: &RunId, prefix: &str, cursor: Option<&str>) -> Result<ScanResult>;

    // Batch (no separate trait)
    fn mget(&self, run: &RunId, keys: &[&str]) -> Result<Vec<Option<Versioned<Value>>>>;
    fn mput(&self, run: &RunId, entries: &[(&str, Value)]) -> Result<Version>;
    fn mdelete(&self, run: &RunId, keys: &[&str]) -> Result<u64>;
}
```

### 6.3 Cleaned Facade Traits

```rust
// Simple, Redis-like
pub trait KV {
    fn get(&self, key: &str) -> Result<Option<Value>>;
    fn set(&self, key: &str, value: Value) -> Result<()>;
    fn del(&self, key: &str) -> Result<bool>;
    fn exists(&self, key: &str) -> Result<bool>;
    fn incr(&self, key: &str) -> Result<i64>;
    fn keys(&self, prefix: &str) -> Result<Vec<String>>;
    fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Value>>>;
    fn mset(&self, entries: &[(&str, Value)]) -> Result<()>;
}

// With version info when needed
pub trait KVVersioned: KV {
    fn getv(&self, key: &str) -> Result<Option<Versioned<Value>>>;
    fn history(&self, key: &str, limit: usize) -> Result<Vec<Versioned<Value>>>;
}
```

### 6.4 Minimal Public Types

**Expose:**
- `Strata` (Database alias)
- `StrataBuilder` (DatabaseBuilder alias)
- `RunId` (ApiRunId renamed)
- `RunInfo`, `RunState`
- `Value`, `Versioned<T>`, `Version`, `Timestamp`
- `Error`, `Result<T>`
- All trait types (KVStore, JsonStore, etc.)
- Distance metrics, search filters for VectorStore

**Hide:**
- `SubstrateImpl`, `FacadeImpl`
- `Key`, `Namespace`, `TypeTag`
- `RunId` (internal UUID-based)
- `TxnId`, `TxnInfo` (managed internally)
- All `*Options` structs (use builder pattern)

---

## Decision Checklist

- [ ] Consolidate `Error` and `StrataError` into single error type
- [ ] Rename `ApiRunId` to `RunId` (hide internal RunId)
- [ ] Merge batch traits into main traits
- [ ] Standardize naming: all Facade methods without prefixes, all Substrate with
- [ ] Hide `SubstrateImpl` and `FacadeImpl`
- [ ] Create unified `strata` crate as entry point
- [ ] Remove TTL-related APIs until TTL is implemented
- [ ] Add missing stream listing to EventLog
- [ ] Standardize return types (all mutations return `Version`)

---

## Appendix: Current vs Proposed

| Current | Proposed | Change |
|---------|----------|--------|
| `strata_api::substrate::ApiRunId` | `strata::RunId` | Rename, re-path |
| `strata_engine::Database` | `strata::Strata` | Alias |
| `KVStoreBatch` trait | Merge into `KVStore` | Remove trait |
| `SubstrateImpl` public | `SubstrateImpl` hidden | `#[doc(hidden)]` |
| `xadd`, `vadd` facade methods | `event_add`, `vector_add` | Rename |
| `state_get` returns Versioned | `state_get` returns Value | Strip version |
| Two error types | Single `Error` enum | Consolidate |
