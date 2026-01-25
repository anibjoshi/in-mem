# Unified API Design for Strata

> **Status**: Proposal for Review
> **Author**: Architecture Team
> **Date**: 2026-01-25

---

## Executive Summary

This document proposes a **unified API surface** for Strata with clean, intuitive naming:

```rust
use strata::prelude::*;

let db = Strata::open("./my-db")?;

db.kv.set("key", value)?;           // Simple
db.kv.set_in(&run, "key", value)?;  // With explicit run
db.kv.put(&run, "key", value)?;     // Full control, returns version

db.json.set("doc", "$.name", "Alice")?;
db.events.append("stream", payload)?;
db.vectors.upsert("collection", "id", vec, metadata)?;
```

**Design Principles:**
1. One API surface (not separate "Facade" and "Substrate")
2. Simple names: `KV`, `Json`, `Events`, `State`, `Vectors`, `Runs`
3. No redundant prefixes: `db.json.set()` not `db.json.json_set()`
4. Progressive disclosure: simple methods → run-scoped → full control

---

## API Structure

### Entry Point

```rust
use strata::prelude::*;

let db = Strata::open("./my-db")?;

// Access primitives via fields
db.kv        // Key-Value operations
db.json      // JSON document operations
db.events    // Event log operations
db.state     // State cell operations
db.vectors   // Vector similarity search
db.runs      // Run lifecycle management
```

---

## KV (Key-Value)

```rust
db.kv
```

### Simple Operations

```rust
// Reads
db.kv.get("key")?                      // -> Option<Value>
db.kv.get_versioned("key")?            // -> Option<Versioned<Value>>
db.kv.exists("key")?                   // -> bool
db.kv.keys("prefix")?                  // -> Vec<String>
db.kv.scan("prefix", cursor)?          // -> ScanResult

// Writes (default run, auto-commit)
db.kv.set("key", value)?               // -> ()
db.kv.delete("key")?                   // -> bool
db.kv.incr("key")?                     // -> i64
db.kv.incr_by("key", 10)?              // -> i64
db.kv.set_nx("key", value)?            // -> bool (set if not exists)

// Batch
db.kv.mget(&["a", "b", "c"])?          // -> Vec<Option<Value>>
db.kv.mset(&[("a", 1), ("b", 2)])?     // -> ()
db.kv.mdelete(&["a", "b"])?            // -> u64 (count deleted)
```

### Run-Scoped Operations

```rust
db.kv.get_in(&run, "key")?             // -> Option<Value>
db.kv.set_in(&run, "key", value)?      // -> ()
db.kv.delete_in(&run, "key")?          // -> bool
```

### Full Control

```rust
// Returns version
db.kv.put(&run, "key", value)?         // -> Version

// Point-in-time read
db.kv.get_at(&run, "key", version)?    // -> Versioned<Value>

// Compare-and-swap
db.kv.cas(&run, "key", expected_version, value)?  // -> bool

// History
db.kv.history("key", limit)?           // -> Vec<Versioned<Value>>
db.kv.history_in(&run, "key", limit)?  // -> Vec<Versioned<Value>>

// Batch with version
db.kv.mput(&run, &[("a", 1), ("b", 2)])?  // -> Version
```

---

## Json (Document Store)

```rust
db.json
```

### Simple Operations

```rust
// Document CRUD
db.json.get("doc")?                    // -> Option<Value> (whole doc)
db.json.get_path("doc", "$.field")?    // -> Option<Value> (at path)
db.json.set("doc", value)?             // -> () (whole doc)
db.json.set_path("doc", "$.field", value)?  // -> () (at path)
db.json.delete("doc")?                 // -> bool
db.json.delete_path("doc", "$.field")? // -> u64 (count deleted)
db.json.exists("doc")?                 // -> bool

// Merge patch (RFC 7396)
db.json.merge("doc", "$.field", patch)?  // -> ()

// Array operations
db.json.push("doc", "$.items", values)?   // -> usize (new length)
db.json.pop("doc", "$.items")?            // -> Option<Value>
db.json.array_len("doc", "$.items")?      // -> Option<usize>

// Numeric
db.json.incr("doc", "$.count", 1.0)?      // -> f64

// Querying
db.json.query("$.status", "active", 100)? // -> Vec<String> (doc keys)
db.json.search("full text query", 10)?    // -> Vec<SearchHit>
db.json.list("prefix", 100)?              // -> Vec<String>
db.json.count()?                          // -> u64

// Batch
db.json.mget(&["doc1", "doc2"])?          // -> Vec<Option<Value>>
db.json.mcreate(vec![doc1, doc2])?        // -> Vec<String> (generated keys)
```

### Run-Scoped Operations

```rust
db.json.get_in(&run, "doc")?
db.json.set_in(&run, "doc", value)?
db.json.get_path_in(&run, "doc", "$.field")?
db.json.set_path_in(&run, "doc", "$.field", value)?
```

### Full Control

```rust
// With version return
db.json.put(&run, "doc", value)?              // -> Version
db.json.put_path(&run, "doc", "$.f", value)?  // -> Version

// Versioned read
db.json.get_versioned("doc")?                 // -> Option<Versioned<Value>>

// Compare-and-swap
db.json.cas(&run, "doc", expected, "$.f", value)?  // -> bool

// History
db.json.history("doc", limit)?                // -> Vec<Versioned<Value>>
```

---

## Events (Append-Only Log)

```rust
db.events
```

### Simple Operations

```rust
// Append (returns sequence number)
db.events.append("stream", payload)?          // -> u64
db.events.append_batch("stream", payloads)?   // -> Vec<u64>

// Read
db.events.get("stream", sequence)?            // -> Option<Event>
db.events.range("stream", start, end)?        // -> Vec<Event>
db.events.range_limit("stream", start, end, limit)?  // -> Vec<Event>
db.events.latest("stream")?                   // -> Option<u64>
db.events.len("stream")?                      // -> u64

// Stream management
db.events.streams()?                          // -> Vec<String>
```

### Run-Scoped Operations

```rust
db.events.append_in(&run, "stream", payload)?
db.events.range_in(&run, "stream", start, end)?
db.events.get_in(&run, "stream", sequence)?
```

### Full Control

```rust
// With full metadata
db.events.get_versioned("stream", seq)?       // -> Option<Versioned<Value>>
```

### Event Type

```rust
pub struct Event {
    pub sequence: u64,
    pub payload: Value,
    pub timestamp: Timestamp,
}
```

---

## State (CAS Cells)

```rust
db.state
```

### Simple Operations

```rust
// Read/Write
db.state.get("cell")?                  // -> Option<Value>
db.state.set("cell", value)?           // -> ()
db.state.delete("cell")?               // -> bool
db.state.exists("cell")?               // -> bool

// Compare-and-swap (returns success)
db.state.cas("cell", expected_counter, value)?  // -> bool
```

### Run-Scoped Operations

```rust
db.state.get_in(&run, "cell")?
db.state.set_in(&run, "cell", value)?
db.state.cas_in(&run, "cell", expected, value)?
```

### Full Control

```rust
// With version/counter info (needed for CAS)
db.state.get_versioned("cell")?        // -> Option<Versioned<Value>>

// With version return
db.state.put(&run, "cell", value)?     // -> Version

// History
db.state.history("cell", limit)?       // -> Vec<Versioned<Value>>
```

---

## Vectors (Similarity Search)

```rust
db.vectors
```

### Simple Operations

```rust
// CRUD
db.vectors.upsert("coll", "key", vec, metadata)?  // -> ()
db.vectors.get("coll", "key")?                    // -> Option<VectorEntry>
db.vectors.delete("coll", "key")?                 // -> bool
db.vectors.exists("coll", "key")?                 // -> bool

// Search
db.vectors.search("coll", query_vec, k)?          // -> Vec<VectorMatch>
db.vectors.search_filter("coll", query, k, filter)?  // -> Vec<VectorMatch>

// Collection management
db.vectors.collections()?                         // -> Vec<String>
db.vectors.collection_info("coll")?               // -> CollectionInfo
db.vectors.count("coll")?                         // -> u64
```

### Run-Scoped Operations

```rust
db.vectors.upsert_in(&run, "coll", "key", vec, meta)?
db.vectors.search_in(&run, "coll", query, k)?
db.vectors.get_in(&run, "coll", "key")?
```

### Full Control

```rust
// With version return
db.vectors.put(&run, "coll", "key", vec, meta)?   // -> Version

// With source reference (provenance tracking)
db.vectors.put_with_source(&run, "coll", "key", vec, meta, source_ref)?  // -> Version

// Versioned read
db.vectors.get_versioned("coll", "key")?          // -> Option<Versioned<VectorEntry>>

// Search with options
db.vectors.search_with("coll", query, k, options)?  // -> Vec<VectorMatch>
```

### Types

```rust
pub struct VectorEntry {
    pub vector: Vec<f32>,
    pub metadata: Value,
}

pub struct VectorMatch {
    pub key: String,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
    pub metadata: Value,
}

pub struct SearchOptions {
    pub metric: DistanceMetric,
    pub filter: Option<Filter>,
    pub include_vectors: bool,
}

pub enum DistanceMetric {
    Cosine,       // Default
    Euclidean,
    DotProduct,
}

pub enum Filter {
    Eq(String, Value),
    Prefix(String, String),
    Range { field: String, min: Option<f64>, max: Option<f64> },
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
}
```

---

## Runs (Lifecycle Management)

```rust
db.runs
```

### Lifecycle

```rust
// Create
db.runs.create(metadata)?              // -> RunId
db.runs.create_named("my-run", meta)?  // -> RunId

// Read
db.runs.get(&run)?                     // -> Option<RunInfo>
db.runs.exists(&run)?                  // -> bool
db.runs.list(filter, limit)?           // -> Vec<RunInfo>
db.runs.default()?                     // -> RunId

// State transitions
db.runs.close(&run)?                   // -> () (mark completed)
db.runs.pause(&run)?                   // -> ()
db.runs.resume(&run)?                  // -> ()
db.runs.fail(&run, "error message")?   // -> ()
db.runs.cancel(&run)?                  // -> ()
db.runs.archive(&run)?                 // -> () (terminal state)

// Metadata
db.runs.update_metadata(&run, meta)?   // -> ()
```

### Advanced

```rust
// Replay (read-only historical view)
db.runs.replay(&run)?                  // -> ReadOnlyView

// Diff two runs
db.runs.diff(&run_a, &run_b)?          // -> RunDiff

// Export/Import (RunBundle)
db.runs.export(&run, path)?            // -> ()
db.runs.import(path)?                  // -> RunId
```

### Types

```rust
pub struct RunId { /* opaque */ }

impl RunId {
    pub fn new() -> Self;              // Random UUID
    pub fn parse(s: &str) -> Result<Self>;
    pub fn is_default(&self) -> bool;
    pub fn as_str(&self) -> &str;
}

pub struct RunInfo {
    pub id: RunId,
    pub name: Option<String>,
    pub state: RunState,
    pub created_at: Timestamp,
    pub metadata: Value,
    pub error: Option<String>,
}

pub enum RunState {
    Active,
    Completed,
    Failed,
    Cancelled,
    Paused,
    Archived,
}
```

---

## Transactions

```rust
// Execute multiple operations atomically
db.transaction(&run, |tx| {
    tx.kv.set("key1", value1)?;
    tx.kv.set("key2", value2)?;
    tx.events.append("stream", event)?;
    Ok(())
})?;

// With retry on conflict
db.transaction_retry(&run, 3, |tx| {
    let balance = tx.kv.get("balance")?.unwrap_or(Value::Int(0));
    tx.kv.set("balance", balance + 100)?;
    Ok(())
})?;
```

The transaction closure receives a `Transaction` object with the same API:

```rust
pub struct Transaction {
    pub kv: KV,
    pub json: Json,
    pub events: Events,
    pub state: State,
    pub vectors: Vectors,
}
```

---

## Database Lifecycle

```rust
// Open
let db = Strata::open("./my-db")?;

// Or with builder
let db = Strata::builder()
    .path("./my-db")
    .open()?;

// In-memory (for testing)
let db = Strata::builder()
    .in_memory()
    .open()?;

// Lifecycle
db.flush()?                   // Force flush to disk
db.shutdown()?                // Graceful shutdown
db.is_open()                  // -> bool

// Info
db.info()                     // -> DatabaseInfo
```

### Builder Options

```rust
Strata::builder()
    .path("./db")             // Storage path
    .in_memory()              // No persistence (testing)
    .buffered()               // Buffered writes (default, production)
    .strict()                 // Fsync every write (maximum durability)
    .open()?
```

---

## Core Types

### Value

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

// Convenient conversions
impl From<i64> for Value { }
impl From<&str> for Value { }
impl From<String> for Value { }
impl From<bool> for Value { }
impl From<f64> for Value { }
impl From<Vec<u8>> for Value { }
// etc.
```

### Versioned<T>

```rust
pub struct Versioned<T> {
    value: T,
    version: Version,
    timestamp: Timestamp,
}

impl<T> Versioned<T> {
    pub fn value(&self) -> &T;
    pub fn into_value(self) -> T;
    pub fn version(&self) -> Version;
    pub fn timestamp(&self) -> Timestamp;
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Versioned<U>;
}
```

### Version & Timestamp

```rust
pub struct Version { /* opaque */ }

impl Version {
    pub fn as_u64(&self) -> u64;
    pub fn is_zero(&self) -> bool;
}

pub struct Timestamp { /* opaque */ }

impl Timestamp {
    pub fn now() -> Self;
    pub fn as_micros(&self) -> u64;
}
```

### Error

```rust
pub enum Error {
    NotFound(String),
    WrongType { expected: String, actual: String },
    InvalidKey(String),
    InvalidPath(String),
    VersionMismatch { expected: u64, actual: u64 },
    Conflict(String),
    ConstraintViolation(String),
    Io(std::io::Error),
    Serialization(String),
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

---

## Complete Example

```rust
use strata::prelude::*;

fn main() -> Result<()> {
    // Open database
    let db = Strata::open("./my-app-db")?;

    // Simple KV operations (default run)
    db.kv.set("user:1:name", "Alice")?;
    db.kv.set("user:1:score", 100)?;

    let name = db.kv.get("user:1:name")?;
    let score = db.kv.incr("user:1:score")?;

    // JSON documents
    db.json.set("profile:1", json!({
        "name": "Alice",
        "settings": { "theme": "dark" }
    }))?;

    db.json.set_path("profile:1", "$.settings.theme", "light")?;

    // Events
    db.events.append("user:1:activity", json!({
        "action": "login",
        "ip": "192.168.1.1"
    }))?;

    // Vectors
    db.vectors.upsert("embeddings", "doc:1",
        vec![0.1, 0.2, 0.3],
        json!({"title": "My Document"})
    )?;

    let similar = db.vectors.search("embeddings", vec![0.1, 0.2, 0.3], 10)?;

    // State with CAS
    let current = db.state.get_versioned("counter")?;
    if let Some(v) = current {
        db.state.cas("counter", v.version().as_u64(), v.value() + 1)?;
    }

    // Working with runs
    let run = db.runs.create(json!({"agent": "my-agent"}))?;

    db.kv.set_in(&run, "step", 1)?;
    db.events.append_in(&run, "log", json!({"msg": "started"}))?;

    db.runs.close(&run)?;

    // Transactions
    db.transaction(&run, |tx| {
        tx.kv.set("a", 1)?;
        tx.kv.set("b", 2)?;
        Ok(())
    })?;

    // Shutdown
    db.shutdown()?;

    Ok(())
}
```

---

## Naming Conventions Summary

| Primitive | Accessor | Example Methods |
|-----------|----------|-----------------|
| Key-Value | `db.kv` | `get`, `set`, `delete`, `incr`, `cas` |
| JSON | `db.json` | `get`, `set`, `get_path`, `set_path`, `merge`, `push` |
| Events | `db.events` | `append`, `get`, `range`, `len`, `latest` |
| State | `db.state` | `get`, `set`, `cas`, `delete` |
| Vectors | `db.vectors` | `upsert`, `get`, `delete`, `search` |
| Runs | `db.runs` | `create`, `get`, `close`, `pause`, `archive` |

### Method Suffixes

| Suffix | Meaning | Example |
|--------|---------|---------|
| (none) | Simple, default run | `db.kv.get("key")` |
| `_in` | Explicit run | `db.kv.get_in(&run, "key")` |
| `_versioned` | Returns `Versioned<T>` | `db.kv.get_versioned("key")` |
| `_at` | Point-in-time | `db.kv.get_at(&run, "key", version)` |
| `_path` | JSON path operation | `db.json.get_path("doc", "$.field")` |

### Write Method Naming

| Method | Returns | Use Case |
|--------|---------|----------|
| `set` | `()` | Simple write, don't need version |
| `put` | `Version` | Need version for CAS or tracking |

---

## Migration from Current API

| Before | After |
|--------|-------|
| `facade.set("key", v)` | `db.kv.set("key", v)` |
| `substrate.kv_put(&run, "key", v)` | `db.kv.put(&run, "key", v)` |
| `substrate.kv_get(&run, "key")` | `db.kv.get_in(&run, "key")` |
| `facade.json_set("k", "$", v)` | `db.json.set("k", v)` |
| `substrate.event_append(&run, "s", p)` | `db.events.append_in(&run, "s", p)` |
| `facade.xadd("stream", p)` | `db.events.append("stream", p)` |
| `facade.vadd("c", "k", vec, m)` | `db.vectors.upsert("c", "k", vec, m)` |
| `substrate.run_create(id, m)` | `db.runs.create(m)` |

---

## Decision Checklist

- [ ] Approve unified API (no separate Facade/Substrate)
- [ ] Approve accessor pattern (`db.kv`, `db.json`, etc.)
- [ ] Approve naming (no redundant prefixes)
- [ ] Approve method suffix conventions (`_in`, `_versioned`, `_at`, `_path`)
- [ ] Approve `set` vs `put` distinction
- [ ] Approve deprecation plan for old API
