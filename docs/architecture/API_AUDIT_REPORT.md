# Strata API Comprehensive Audit Report

> **Status**: Technical Audit
> **Date**: 2026-01-25
> **Scope**: Complete external API surface analysis

---

## Executive Summary

This audit examines Strata's current API against best-in-class databases:
- **Redis** - KV, JSON, Streams
- **DynamoDB** - Versioned KV, conditions
- **MongoDB** - Document operations
- **Pinecone/Weaviate** - Vector search
- **PostgreSQL** - Transactions
- **FoundationDB** - Layers and isolation

**Critical Issues Found: 47**
- Naming inconsistencies: 18
- Redundant API patterns: 12
- Missing functionality: 9
- Usability issues: 8

---

## Table of Contents

1. [Current API Summary](#1-current-api-summary)
2. [Best-in-Class Comparison](#2-best-in-class-comparison)
3. [Issue Catalog](#3-issue-catalog)
4. [Recommended Changes](#4-recommended-changes)
5. [Priority Matrix](#5-priority-matrix)

---

## 1. Current API Summary

### 1.1 Substrate API (Power User)

| Trait | Methods | Purpose |
|-------|---------|---------|
| `KVStore` | 11 | Key-value CRUD + CAS |
| `KVStoreBatch` | 4 | Batch operations |
| `JsonStore` | 17 | JSON document operations |
| `EventLog` | 11 | Append-only event streams |
| `StateCell` | 11 | CAS cells with transitions |
| `VectorStore` | 19 | Vector similarity search |
| `RunIndex` | 24 | Run lifecycle management |
| `TransactionControl` | 5 | Transaction management |
| `TransactionSavepoint` | 3 | Savepoint operations |
| `RetentionSubstrate` | 3 | Retention policies |
| `RetentionSubstrateExt` | 2 | Retention GC |

**Total: 110 methods across 11 traits**

### 1.2 Facade API (Simple User)

| Trait | Methods | Purpose |
|-------|---------|---------|
| `KVFacade` | 14 | Redis-like KV |
| `KVFacadeBatch` | 4 | Batch operations |
| `JsonFacade` | 13 | RedisJSON-like |
| `EventFacade` | 7 | Redis Streams-like |
| `StateFacade` | 5 | State cells |
| `VectorFacade` | 16 | Vector operations |
| `RunFacade` | 2 | Run listing |
| `HistoryFacade` | 3 | Version history |
| `SystemFacade` | 1 | Capabilities |

**Total: 65 methods across 9 traits**

### 1.3 Overlap Analysis

```
Total Unique Functionality: ~90 operations
Duplicated Between Layers: ~60 operations (67%)
Substrate-Only: ~50 operations
Facade-Only: ~5 operations
```

---

## 2. Best-in-Class Comparison

### 2.1 Redis (KV & Streams)

**Redis Strengths:**
- Single, unified API surface
- Consistent naming: `GET`, `SET`, `DEL`, `EXISTS`
- Options via flags: `SET key value NX EX 3600`
- Batch via `MGET`, `MSET`
- Streams: `XADD`, `XREAD`, `XRANGE`

**Strata Issues vs Redis:**

| Issue | Redis | Strata Current | Problem |
|-------|-------|----------------|---------|
| Naming | `GET` | `kv_get` / `get` | Redundant prefix |
| Options | Flags in command | Separate `*Options` structs | Over-engineered |
| Increment | `INCR`, `INCRBY` | `incr`, `incrby`, `incr_with_options` | Too many variants |
| Streams | `XADD` | `xadd` / `event_append` | Two different names |
| Consistency | All UPPERCASE | Mixed `snake_case` | N/A (Rust convention) |

### 2.2 DynamoDB (Versioned KV)

**DynamoDB Strengths:**
- Condition expressions: `attribute_not_exists(pk)`
- Update expressions: `SET #count = #count + :val`
- Consistent return: always returns consumed capacity
- Transaction API: `TransactWriteItems`, `TransactGetItems`

**Strata Issues vs DynamoDB:**

| Issue | DynamoDB | Strata Current | Problem |
|-------|----------|----------------|---------|
| Conditions | Expression language | Separate CAS methods | Less flexible |
| Updates | Expression language | Separate `incr` methods | Less composable |
| Returns | Always returns metadata | Some return `()` | Inconsistent |

### 2.3 MongoDB (Documents)

**MongoDB Strengths:**
- Unified query language
- Aggregation pipeline
- Consistent: `find`, `findOne`, `insertOne`, `insertMany`
- Path notation: `field.nested.value`

**Strata Issues vs MongoDB:**

| Issue | MongoDB | Strata Current | Problem |
|-------|---------|----------------|---------|
| Path syntax | Dot notation | JSONPath `$.field` | More complex |
| Batch naming | `insertMany` | `json_batch_create` | Inconsistent with KV `mput` |
| Query | Aggregation | `json_query`, `json_search` | Two separate concepts |

### 2.4 Pinecone (Vectors)

**Pinecone Strengths:**
- Simple: `upsert`, `query`, `delete`, `fetch`
- Namespaces (like our collections)
- Metadata filtering in query
- Sparse-dense hybrid search

**Strata Issues vs Pinecone:**

| Issue | Pinecone | Strata Current | Problem |
|-------|----------|----------------|---------|
| Method names | `query` | `vector_search` / `vsim` | Two names, neither is `query` |
| Fetch | `fetch` | `vector_get` / `vget` | Inconsistent |
| Upsert return | Returns upserted count | Returns `Version` or `()` | Inconsistent |

### 2.5 PostgreSQL (Transactions)

**PostgreSQL Strengths:**
- Simple: `BEGIN`, `COMMIT`, `ROLLBACK`
- Savepoints: `SAVEPOINT name`, `ROLLBACK TO name`
- Isolation levels
- Clear error handling

**Strata Issues vs PostgreSQL:**

| Issue | PostgreSQL | Strata Current | Problem |
|-------|------------|----------------|---------|
| Naming | `BEGIN` | `txn_begin` | Unnecessary prefix |
| Exposure | Direct commands | Trait with options | Over-abstracted |
| Usage | Implicit in connection | Explicit everywhere | More verbose |

### 2.6 FoundationDB (Layers)

**FoundationDB Strengths:**
- Clean layer abstraction
- Transactions are first-class
- Directory layer for namespacing
- Atomic operations built-in

**Strata Alignment:**
- Run concept similar to Directory layer ✓
- Versioning similar to MVCC ✓
- But: No clear layer separation in API

---

## 3. Issue Catalog

### Category A: Naming Inconsistencies (18 issues)

#### A1. Redundant Prefixes on Methods

| Current | Should Be | Location |
|---------|-----------|----------|
| `kv_get` | `get` | Substrate KVStore |
| `kv_put` | `put` | Substrate KVStore |
| `kv_delete` | `delete` | Substrate KVStore |
| `json_set` | `set` | Substrate JsonStore |
| `json_get` | `get` | Substrate JsonStore |
| `event_append` | `append` | Substrate EventLog |
| `state_set` | `set` | Substrate StateCell |
| `state_get` | `get` | Substrate StateCell |
| `vector_upsert` | `upsert` | Substrate VectorStore |
| `vector_search` | `search` | Substrate VectorStore |
| `run_create` | `create` | Substrate RunIndex |
| `txn_begin` | `begin` | TransactionControl |

**Impact**: Verbose, redundant since accessed via `db.kv.get()` not `kv_get()`

#### A2. Inconsistent Facade Naming

| Primitive | Facade Methods | Issue |
|-----------|----------------|-------|
| KV | `get`, `set`, `del` | Good |
| JSON | `json_get`, `json_set` | Still has prefix! |
| Events | `xadd`, `xrange`, `xlen` | Redis-style, inconsistent |
| State | `state_get`, `state_set` | Still has prefix! |
| Vectors | `vadd`, `vget`, `vsim` | `v` prefix, not `vector_` |

**Impact**: Mixed naming conventions confuse users

#### A3. Batch Method Naming

| Primitive | Current | Inconsistency |
|-----------|---------|---------------|
| KV Substrate | `kv_mget`, `kv_mput` | `m` prefix |
| KV Facade | `mget`, `mset`, `mdel` | `m` prefix |
| JSON Substrate | `json_batch_get`, `json_batch_create` | `batch_` prefix |
| Vector Substrate | `vector_upsert_batch`, `vector_get_batch` | `_batch` suffix |
| Vector Facade | `vadd_batch`, `vget_batch` | `_batch` suffix |

**Impact**: Three different batch naming conventions

#### A4. History Method Naming

| Primitive | Current | Issue |
|-----------|---------|-------|
| KV | `kv_history` | OK |
| JSON | `json_history` | OK |
| State | `state_history` | OK |
| Vector | `vector_history` | OK |
| Facade | `history` (HistoryFacade) | Only for KV, not others |

**Impact**: Facade history only works for KV

#### A5. Existence Check Naming

| Primitive | Current | Issue |
|-----------|---------|-------|
| KV | `kv_exists` | OK |
| JSON | `json_exists` | OK |
| State | `state_exists` | OK |
| Vector | `vector_collection_exists` | Different pattern |
| Run | `run_exists` | OK |

**Impact**: Vector uses `collection_exists` instead of just `exists`

#### A6. Count Method Naming

| Primitive | Current | Issue |
|-----------|---------|-------|
| JSON | `json_count` | Document count |
| Vector | `vector_count` | Vector count in collection |
| Run | `run_count` | Run count |
| KV | (missing) | No key count |
| State | (missing) | No cell count |
| Events | `event_len` | Different name (`len` not `count`) |

**Impact**: Inconsistent naming and missing methods

---

### Category B: Redundant API Patterns (12 issues)

#### B1. Two API Layers (Facade + Substrate)

**Problem**: Users must choose between two APIs with different:
- Method names (`get` vs `kv_get`)
- Parameters (implicit run vs explicit run)
- Return types (`Value` vs `Versioned<Value>`)

**Best Practice**: Single API with progressive disclosure

#### B2. Separate Batch Traits

| Current | Issue |
|---------|-------|
| `KVStore` + `KVStoreBatch` | Two traits for one primitive |
| `KVFacade` + `KVFacadeBatch` | Two traits for one primitive |

**Best Practice**: Merge batch methods into main trait

#### B3. Options Struct Proliferation

| Struct | Used By | Issue |
|--------|---------|-------|
| `GetOptions` | `get_with_options` | Could be method variants |
| `SetOptions` | `set_with_options` | Could be method variants |
| `IncrOptions` | `incr_with_options` | Only has `initial` field |
| `RangeOptions` | N/A (defined but unused?) | Dead code? |
| `TxnOptions` | `txn_begin` | Only 2 fields |
| `VectorSearchOptions` | `vsim_with_options` | OK, legitimate |

**Best Practice**: Use method variants or builder pattern

#### B4. Duplicate Versioned Types

| Type | Location | Issue |
|------|----------|-------|
| `Versioned<T>` | `strata_core::contract` | Core type |
| `Versioned<T>` | `facade::kv` | Duplicate! |
| `VersionedValue` | `facade::history` | Third version! |
| `StateValue` | `facade::state` | Yet another variant |

**Best Practice**: Single `Versioned<T>` type

#### B5. Multiple Return Patterns for Same Operation

| Operation | Substrate Return | Facade Return |
|-----------|-----------------|---------------|
| Put/Set | `Version` | `()` |
| Get | `Option<Versioned<Value>>` | `Option<Value>` |
| Delete | `bool` | `bool` |
| Incr | `i64` | `i64` |
| CAS | `bool` or `Option<Version>` | `Option<u64>` |

**Issue**: CAS returns different types in different contexts

---

### Category C: Missing Functionality (9 issues)

#### C1. Missing Primitive Operations

| Primitive | Missing | Redis/Other Has |
|-----------|---------|-----------------|
| KV | `keys_count` | `DBSIZE` |
| KV | `type` | `TYPE key` |
| KV | `ttl` / `expire` | `TTL`, `EXPIRE` |
| KV | `rename` | `RENAME` |
| Events | `stream_delete` | `XDEL` |
| Events | `stream_trim` | `XTRIM` |
| State | `list` with cursor | Cursor-based scan |
| JSON | `json_keys` | Like `KEYS` for JSON docs |

#### C2. Missing Convenience Methods

| Missing | Would Be | Rationale |
|---------|----------|-----------|
| `get_or_default` | `db.kv.get_or("key", default)` | Common pattern |
| `set_if_changed` | `db.kv.set_if_changed(k, v)` | Skip if same value |
| `update` | `db.kv.update("key", \|v\| v+1)` | Atomic update |
| `pop` | `db.kv.pop("key")` | Get and delete |

#### C3. Missing Bulk Operations

| Missing | Description |
|---------|-------------|
| `clear_run` | Delete all data in a run |
| `copy_run` | Copy run data to new run |
| `merge_runs` | Merge multiple runs |

#### C4. Missing Observability

| Missing | Description |
|---------|-------------|
| `watch` | Watch key for changes |
| `subscribe` | Subscribe to event streams |
| `on_change` | Callback on data change |

---

### Category D: Usability Issues (8 issues)

#### D1. Verbose Run Parameter

**Current**:
```rust
substrate.kv_put(&run, "key", value)?;
substrate.kv_get(&run, "key")?;
substrate.kv_delete(&run, "key")?;
```

**Issue**: Every single call requires `&run`

**Better**:
```rust
let scoped = db.run(&run);
scoped.kv.put("key", value)?;
scoped.kv.get("key")?;
```

#### D2. No Fluent/Chaining API

**Current**:
```rust
db.kv.set("a", 1)?;
db.kv.set("b", 2)?;
db.kv.set("c", 3)?;
```

**Better** (optional):
```rust
db.kv.set("a", 1)
    .set("b", 2)
    .set("c", 3)
    .commit()?;
```

#### D3. Confusing CAS Semantics

**Current StateCell**:
```rust
// Returns Option<Version> - None means CAS failed
state_cas(&run, "cell", Some(5), value) -> Option<Version>
```

**Issue**: `Option<Version>` is confusing - did it succeed or not?

**Better**:
```rust
// Returns Result with specific error
state_cas(&run, "cell", 5, value) -> Result<Version, CasError>
```

#### D4. History Pagination Inconsistency

| Method | Pagination Style |
|--------|------------------|
| `kv_history` | `limit, before: Option<Version>` |
| `kv_scan` | `limit, cursor: Option<&str>` |
| `json_list` | `cursor: Option<&str>, limit` |
| `vector_list_keys` | `limit: Option<usize>, cursor: Option<&str>` |

**Issue**: Parameter order varies, some use `before`, some use `cursor`

#### D5. Inconsistent Optional Parameters

| Method | Style |
|--------|-------|
| `kv_keys(run, prefix, limit: Option<usize>)` | Option for limit |
| `kv_scan(run, prefix, limit: usize, cursor)` | Required limit |
| `json_list(run, prefix: Option<&str>, cursor, limit: u64)` | Option for prefix |

**Issue**: Sometimes `Option`, sometimes required, different types (`usize` vs `u64`)

#### D6. Leaky Abstractions

| Type | Issue |
|------|-------|
| `ApiRunId::to_run_id()` | Exposes internal `RunId` |
| `Version` has `txn()`, `seq()`, `counter()` | Exposes internal version types |
| `TxnId(u64)` | Should be opaque |

#### D7. Error Type Confusion

| Type | Location | Use |
|------|----------|-----|
| `Error` | `strata_core::error` | Internal |
| `StrataError` | `strata_core::error` | API |
| `StrataResult<T>` | API methods | Result alias |
| `Result<T>` | Some methods | std Result |

**Issue**: Two error types, unclear which to use

#### D8. No Type-Safe Keys

**Current**:
```rust
db.kv.set("user:123", value)?;  // Just a string
db.kv.set("user:abc", value)?;  // Typo? Different type?
```

**Better** (optional type-safe API):
```rust
let key = UserKey::new(123);
db.kv.set(key, value)?;
```

---

## 4. Recommended Changes

### 4.1 Unified API Structure

**Before**:
```
strata_api
├── substrate/           # Power user
│   ├── KVStore
│   ├── JsonStore
│   └── ...
└── facade/              # Simple user
    ├── KVFacade
    ├── JsonFacade
    └── ...
```

**After**:
```
strata
├── Strata (main entry)
├── KV (db.kv)
├── Json (db.json)
├── Events (db.events)
├── State (db.state)
├── Vectors (db.vectors)
└── Runs (db.runs)
```

### 4.2 Method Naming Cleanup

**Pattern**: No prefixes, access via primitive

```rust
// Before
substrate.kv_get(&run, "key")?;
facade.json_set("key", "$", value)?;

// After
db.kv.get("key")?;
db.json.set("key", value)?;
```

**Batch Naming**: Consistent `m*` prefix

```rust
db.kv.mget(&["a", "b"])?;
db.kv.mset(&[("a", 1), ("b", 2)])?;
db.json.mget(&["doc1", "doc2"])?;
db.vectors.mget("coll", &["k1", "k2"])?;
```

### 4.3 Consistent Method Signatures

**Writes**:
| Simple | Full |
|--------|------|
| `set(key, value) -> ()` | `put(run, key, value) -> Version` |

**Reads**:
| Simple | Versioned |
|--------|-----------|
| `get(key) -> Option<Value>` | `get_versioned(key) -> Option<Versioned<Value>>` |

**Run-Scoped**:
| Pattern | Example |
|---------|---------|
| `*_in(run, ...)` | `get_in(&run, "key")` |

### 4.4 Simplified Options

**Before**:
```rust
set_with_options("key", value, SetOptions::new().nx().get())?;
```

**After**:
```rust
set_nx("key", value)?;        // Set if not exists
set_xx("key", value)?;        // Set if exists
set_get("key", value)?;       // Set and return old
```

### 4.5 Consistent Pagination

**Standard Pattern**:
```rust
fn list(&self, prefix: &str, limit: usize, cursor: Option<&str>) -> Result<Page<T>>;

struct Page<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}
```

### 4.6 Single Error Type

```rust
pub enum Error {
    // Lookup errors
    NotFound { entity: String },

    // Type errors
    WrongType { expected: String, actual: String },

    // Validation errors
    InvalidKey { key: String, reason: String },
    InvalidPath { path: String, reason: String },

    // Concurrency errors
    VersionMismatch { expected: u64, actual: u64 },
    Conflict { reason: String },

    // System errors
    Io(std::io::Error),
    Internal(String),
}
```

### 4.7 Consolidated Types

**Keep**:
- `Value` - unified value enum
- `Versioned<T>` - single versioned wrapper
- `Version` - opaque version (hide internals)
- `Timestamp` - opaque timestamp
- `RunId` - opaque run identifier

**Remove/Hide**:
- `VersionedValue` (use `Versioned<Value>`)
- `StateValue` (use `Versioned<Value>`)
- `ApiRunId` (rename to `RunId`)
- `TxnId` internals (make opaque)

---

## 5. Priority Matrix

### P0: Critical (Do First)

| Change | Impact | Effort |
|--------|--------|--------|
| Unify API layers | High | Medium |
| Remove method prefixes | High | Low |
| Consistent batch naming | Medium | Low |
| Single error type | High | Medium |
| Consolidate versioned types | Medium | Low |

### P1: Important (Do Second)

| Change | Impact | Effort |
|--------|--------|--------|
| Merge batch traits | Medium | Low |
| Consistent pagination | Medium | Medium |
| Remove options structs | Medium | Low |
| Add missing methods | Medium | Medium |

### P2: Nice to Have (Do Later)

| Change | Impact | Effort |
|--------|--------|--------|
| Fluent API | Low | Medium |
| Type-safe keys | Low | High |
| Watch/Subscribe | High | High |
| Run-scoped handle | Medium | Medium |

---

## Appendix: Method Count Reduction

### Before (175+ methods)

```
Substrate: 110 methods
Facade: 65 methods
Overlap: 60+ methods (duplicates)
```

### After (Estimated 80-90 methods)

```
KV: 15 methods
Json: 18 methods
Events: 10 methods
State: 10 methods
Vectors: 15 methods
Runs: 15 methods
Core: 5 methods
```

**Reduction**: ~50% fewer methods, 100% less confusion

---

## Appendix: Naming Reference

### Final Method Names (No Prefixes)

| Primitive | Methods |
|-----------|---------|
| `db.kv` | `get`, `set`, `delete`, `exists`, `incr`, `cas`, `put`, `keys`, `scan`, `mget`, `mset`, `mdelete`, `history` |
| `db.json` | `get`, `set`, `delete`, `exists`, `get_path`, `set_path`, `delete_path`, `merge`, `push`, `pop`, `incr`, `query`, `search`, `list`, `mget` |
| `db.events` | `append`, `get`, `range`, `len`, `latest`, `streams`, `head` |
| `db.state` | `get`, `set`, `delete`, `exists`, `cas`, `put`, `list`, `history` |
| `db.vectors` | `upsert`, `get`, `delete`, `exists`, `search`, `count`, `collections`, `collection_info`, `create_collection`, `drop_collection`, `mget`, `mput`, `mdelete` |
| `db.runs` | `create`, `get`, `exists`, `list`, `close`, `pause`, `resume`, `fail`, `cancel`, `archive`, `delete`, `update_metadata` |

### Suffix Conventions

| Suffix | Meaning |
|--------|---------|
| (none) | Simple, default run |
| `_in` | Explicit run scope |
| `_versioned` | Returns `Versioned<T>` |
| `_at` | Point-in-time read |
| `_path` | JSON path operation |
| `_nx` | Only if not exists |
| `_xx` | Only if exists |
