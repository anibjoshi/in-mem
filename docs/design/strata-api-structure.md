# Strata API Structure

## Overview

This document defines the layered API structure for Strata, the data substrate for AI agents. It establishes the mental model, concurrency semantics, and API design patterns.

## Mental Model: Git Analogy

| Git Concept | Strata Equivalent | Description |
|-------------|-------------------|-------------|
| Repository | `Database` | Shared storage, thread-safe, opened once |
| Working Directory | `Strata` | Per-agent instance with current run context |
| Branch | `Run` | Isolated namespace for data |
| HEAD | `current_run` | The run that operations target |
| `main` branch | Default run | Auto-created, used when no run specified |

```
┌─────────────────────────────────────────────────────────────────┐
│  Database (shared, Arc-wrapped, thread-safe)                    │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ Strata (Agent1) │  │ Strata (Agent2) │  │ Strata (Agent3) │ │
│  │ current: run-1  │  │ current: run-2  │  │ current: run-1  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
│           │                   │                   │             │
│           ▼                   ▼                   ▼             │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Runs (isolated namespaces)               ││
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        ││
│  │  │ default │  │  run-1  │  │  run-2  │  │  run-3  │  ...   ││
│  │  │  (main) │  │         │  │         │  │         │        ││
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘        ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Architecture Layers

### Layer 1: Database (strata-engine)

The `Database` is the shared storage layer. It is:
- **Opened once** per path (singleton pattern with global registry)
- **Thread-safe** (uses `DashMap`, atomics, internal locking)
- **Shared via `Arc<Database>`** across all agents

```rust
// Database is opened once and shared
let database = Database::open("/path/to/data")?;  // Returns Arc<Database>

// Multiple opens of same path return same instance
let db1 = Database::open("/path/to/data")?;  // Same Arc
let db2 = Database::open("/path/to/data")?;  // Same Arc
assert!(Arc::ptr_eq(&db1, &db2));
```

**Implementation: Global Registry**

```rust
use std::sync::{Mutex, Weak};
use std::collections::HashMap;
use std::path::PathBuf;
use once_cell::sync::Lazy;

static OPEN_DATABASES: Lazy<Mutex<HashMap<PathBuf, Weak<Database>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Arc<Database>> {
        let path = path.as_ref().canonicalize()?;
        let mut registry = OPEN_DATABASES.lock().unwrap();

        // Return existing instance if available
        if let Some(weak) = registry.get(&path) {
            if let Some(db) = weak.upgrade() {
                return Ok(db);
            }
        }

        // Create new instance
        let db = Arc::new(Self::open_internal(&path)?);
        registry.insert(path, Arc::downgrade(&db));
        Ok(db)
    }
}
```

### Layer 2: Strata (strata-executor)

`Strata` is the user-facing API. Each agent gets their own instance, like a git working directory.

```rust
pub struct Strata {
    executor: Executor,           // Wraps Arc<Database>
    current_run: Option<RunId>,   // Per-instance context (like HEAD)
}
```

Key properties:
- **Per-agent instance**: Each agent creates their own `Strata`
- **Independent run context**: `set_run()` only affects that instance
- **Not shared across threads**: Use separate instances per thread/agent

### Layer 3: Primitives

Primitives (KV, State, Event, JSON, Vector) are accessed through `Strata`:

```rust
// All primitives accessed through db, operate on current run
db.kv_put("key", value)?;
db.state_set("cell", value)?;
db.event_append("stream", payload)?;
```

## API Design

### Run Management (Git-Like Commands)

| Git Command | Strata API | Description |
|-------------|------------|-------------|
| `git branch <name>` | `db.create_run("name")` | Create a new run |
| `git switch <name>` | `db.set_run("name")` | Switch to existing run |
| `git checkout -b <name>` | `db.checkout_run("name")` | Create if needed, switch to it |
| `git branch` | `db.list_runs()` | List all runs |
| `git branch -d <name>` | `db.delete_run("name")` | Delete a run |
| (implicit) | `db.current_run()` | Get current run name |

### Core API

```rust
impl Strata {
    // === Construction ===

    /// Create a new Strata instance wrapping the given database.
    /// Each agent should create their own instance.
    pub fn new(db: Arc<Database>) -> Self;

    /// Convenience: Open database and create Strata in one call.
    /// Safe to call multiple times with same path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    // === Run Management ===

    /// Create a new run. Does not switch to it.
    pub fn create_run(&self, name: &str) -> Result<RunInfo>;

    /// Switch to an existing run. Returns error if run doesn't exist.
    pub fn set_run(&mut self, name: &str) -> Result<()>;

    /// Create run if it doesn't exist, then switch to it.
    /// Like `git checkout -b`.
    pub fn checkout_run(&mut self, name: &str) -> Result<RunInfo>;

    /// Get the current run name. None if using default run.
    pub fn current_run(&self) -> Option<&str>;

    /// List all runs.
    pub fn list_runs(&self) -> Result<Vec<RunInfo>>;

    /// Delete a run. Cannot delete the default run.
    pub fn delete_run(&self, name: &str) -> Result<()>;

    // === Primitives (operate on current run) ===

    // KV
    pub fn kv_get(&self, key: &str) -> Result<Option<Value>>;
    pub fn kv_put(&self, key: &str, value: Value) -> Result<u64>;
    pub fn kv_delete(&self, key: &str) -> Result<bool>;
    pub fn kv_list(&self, prefix: Option<&str>) -> Result<Vec<String>>;

    // State
    pub fn state_read(&self, name: &str) -> Result<Option<VersionedValue>>;
    pub fn state_set(&self, name: &str, value: Value) -> Result<u64>;
    pub fn state_cas(&self, name: &str, expected: u64, value: Value) -> Result<u64>;

    // Event
    pub fn event_append(&self, stream: &str, payload: Value) -> Result<u64>;
    pub fn event_read(&self, seq: u64) -> Result<Option<Event>>;
    pub fn event_range(&self, stream: Option<&str>, start: Option<u64>, end: Option<u64>, limit: Option<u64>) -> Result<Vec<Event>>;

    // JSON
    pub fn json_get(&self, doc_id: &str, path: &str) -> Result<Option<JsonValue>>;
    pub fn json_set(&self, doc_id: &str, path: &str, value: JsonValue) -> Result<u64>;
    pub fn json_delete(&self, doc_id: &str, path: &str) -> Result<bool>;

    // Vector
    pub fn vector_create_collection(&self, name: &str, dimension: u64, metric: DistanceMetric) -> Result<()>;
    pub fn vector_upsert(&self, collection: &str, key: &str, embedding: Vec<f32>, metadata: Option<Value>) -> Result<()>;
    pub fn vector_search(&self, collection: &str, query: Vec<f32>, k: u64) -> Result<Vec<VectorMatch>>;
    pub fn vector_delete(&self, collection: &str, key: &str) -> Result<bool>;
}
```

## Concurrency Model

### Multiple Agents, Same Database

```rust
let database = Database::open("/data")?;

// Agent 1 - operates on run-1
let mut agent1_db = Strata::new(database.clone());
agent1_db.checkout_run("agent-1-session")?;

// Agent 2 - operates on run-2
let mut agent2_db = Strata::new(database.clone());
agent2_db.checkout_run("agent-2-session")?;

// Concurrent operations are safe - different runs are isolated
std::thread::spawn(move || {
    agent1_db.kv_put("status", "working".into())?;
});
std::thread::spawn(move || {
    agent2_db.kv_put("status", "also working".into())?;
});
```

### Multiple Agents, Same Run

Multiple agents can operate on the same run. Transaction isolation ensures correctness:

```rust
let database = Database::open("/data")?;

let mut agent1_db = Strata::new(database.clone());
agent1_db.set_run("shared-run")?;

let mut agent2_db = Strata::new(database.clone());
agent2_db.set_run("shared-run")?;  // Same run as agent1

// Both agents writing to same run - transactions prevent conflicts
agent1_db.kv_put("counter", 1.into())?;
agent2_db.kv_put("counter", 2.into())?;  // Last write wins (or use CAS for coordination)
```

### Thread Safety Rules

1. **`Database`**: Thread-safe, share via `Arc<Database>`
2. **`Strata`**: NOT thread-safe, create one per thread/agent
3. **Runs**: Isolated namespaces, safe for concurrent access from different `Strata` instances
4. **Same path, multiple opens**: Returns same `Arc<Database>` (safe)

## Changes Required

### 1. Database: Add Global Registry

File: `crates/engine/src/database.rs`

- Add static `OPEN_DATABASES` registry
- Modify `Database::open()` to use registry
- Return `Arc<Database>` instead of `Database`

### 2. Strata: Add Run Context

File: `crates/executor/src/api/mod.rs`

- Add `current_run: Option<RunId>` field
- Add `set_run()`, `create_run()`, `checkout_run()`, `current_run()`, `list_runs()`, `delete_run()`
- Modify all primitive methods to use `current_run` when building commands

### 3. Commands: Keep run: Option<RunId>

The command structure stays the same. `Strata` fills in `run: Some(self.current_run)` when building commands.

### 4. Remove Primitive Constructors from Public API

Users should not directly construct `KVStore`, `EventLog`, etc. They access primitives through `Strata`:

```rust
// OLD (don't expose)
let kv = KVStore::new(db.clone());
kv.put(&run_id, "key", value)?;

// NEW (preferred)
let db = Strata::new(database);
db.kv_put("key", value)?;
```

## Migration Path

### Phase 1: Add Run Context to Strata
- Add `current_run` field
- Add `set_run()`, `checkout_run()`, `current_run()`
- All primitive methods use `current_run`

### Phase 2: Add Database Registry
- Implement global registry for `Database::open()`
- Add `Strata::open()` convenience method

### Phase 3: Simplify Primitive API
- Make primitive constructors `pub(crate)`
- Document that primitives are accessed through `Strata`

### Phase 4: Update Tests and Documentation
- Update all examples to use new pattern
- Add concurrency tests

## Examples

### Simple Usage (Default Run)

```rust
let db = Strata::open("/path/to/data")?;

// All operations go to default run
db.kv_put("config", json!({"debug": true}))?;
let config = db.kv_get("config")?;
```

### Multi-Agent Usage

```rust
let database = Database::open("/shared/data")?;

// Agent 1
let mut db1 = Strata::new(database.clone());
db1.checkout_run("customer-support-agent-run-1")?;
db1.kv_put("context", customer_data)?;
db1.event_append("actions", json!({"action": "lookup"}))?;

// Agent 2 (different run)
let mut db2 = Strata::new(database.clone());
db2.checkout_run("research-agent-run-1")?;
db2.kv_put("context", research_query)?;

// Agent 3 (forking from Agent 1's run for comparison)
let mut db3 = Strata::new(database.clone());
db3.fork_run("customer-support-agent-run-1", "experimental-approach")?;
db3.set_run("experimental-approach")?;
// Now has copy of Agent 1's data, can modify independently
```

### Session/Transaction Usage

```rust
let db = Strata::open("/data")?;
db.checkout_run("my-run")?;

// Multi-operation transaction
db.transaction(|txn| {
    let counter = txn.kv_get("counter")?.unwrap_or(0);
    txn.kv_put("counter", counter + 1)?;
    txn.event_append("increments", json!({"new_value": counter + 1}))?;
    Ok(())
})?;
```

## Open Questions

1. **Default run name**: Should it be "default", "main", or something else?

2. **Auto-create on set_run?**: Should `set_run("foo")` create the run if it doesn't exist, or require explicit `create_run()`/`checkout_run()`?

3. **Run persistence**: Should the "current run" be persisted across restarts, or always start at default?

4. **Strata cloning**: Should `Strata` be `Clone`? If so, clones share `current_run` or get independent context?
