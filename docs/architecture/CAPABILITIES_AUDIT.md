# Strata Capabilities Audit

> **Status**: Complete Audit
> **Date**: 2026-01-25
> **Scope**: All user-facing capabilities

---

## Overview

This document catalogs **everything a user can do** with Strata, organized by category. Each capability includes:
- What the user can do
- Current API (if exists)
- Proposed unified API
- Status (Implemented / Missing / Needs Cleanup)

---

## 1. Database Lifecycle

### 1.1 Opening & Configuration

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Open database at path | `Database::open(path)` | `Strata::open(path)` | ✅ Rename |
| Open with builder | `Database::builder()` | `Strata::builder()` | ✅ Rename |
| Set storage path | `.path(p)` | `.path(p)` | ✅ OK |
| In-memory mode | `.in_memory()` | `.in_memory()` | ✅ OK |
| Buffered durability | `.buffered()` | `.buffered()` | ✅ OK |
| Custom buffered | `.buffered_with(ms, n)` | `.buffered_with(ms, n)` | ✅ OK |
| Strict durability | `.strict()` | `.strict()` | ✅ OK |
| Explicit durability | `.durability(mode)` | `.durability(mode)` | ✅ OK |
| Open configured | `.open()` | `.open()` | ✅ OK |
| Open temporary | `.open_temp()` | `.open_temp()` | ✅ OK |

### 1.2 Durability Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `InMemory` | No persistence, fastest | Testing, caches |
| `Buffered` | Batch writes, ~100ms window | Default production |
| `Strict` | Fsync every write | Maximum durability |

### 1.3 Database Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Force flush to disk | `db.flush()` | `db.flush()` | ✅ OK |
| Graceful shutdown | `db.shutdown()` | `db.shutdown()` | ✅ OK |
| Check if open | `db.is_open()` | `db.is_open()` | ✅ OK |
| Get data directory | `db.data_dir()` | `db.data_dir()` | ✅ OK |
| Get durability mode | `db.durability_mode()` | `db.durability_mode()` | ✅ OK |
| Get database info | (missing) | `db.info()` | ❌ Add |
| Get storage stats | (missing) | `db.stats()` | ❌ Add |

---

## 2. Run Management

### 2.1 Run Lifecycle

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create run | `run_create(run_id, meta)` | `db.runs.create(meta)` | ✅ Simplify |
| Create named run | `run_create(Some(id), meta)` | `db.runs.create_named(name, meta)` | ✅ Add variant |
| Get run info | `run_get(run_id)` | `db.runs.get(&run)` | ✅ OK |
| Check run exists | `run_exists(run_id)` | `db.runs.exists(&run)` | ✅ OK |
| List runs | `run_list(state, limit, offset)` | `db.runs.list(filter, limit)` | ✅ OK |
| Get default run | `ApiRunId::default_run_id()` | `db.runs.default()` | ✅ OK |

### 2.2 Run State Transitions

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Close run (complete) | `run_close(run_id)` | `db.runs.close(&run)` | ✅ OK |
| Pause run | `run_pause(run_id)` | `db.runs.pause(&run)` | ✅ OK |
| Resume run | `run_resume(run_id)` | `db.runs.resume(&run)` | ✅ OK |
| Fail run | `run_fail(run_id, error)` | `db.runs.fail(&run, error)` | ✅ OK |
| Cancel run | `run_cancel(run_id)` | `db.runs.cancel(&run)` | ✅ OK |
| Archive run | `run_archive(run_id)` | `db.runs.archive(&run)` | ✅ OK |
| Delete run | `run_delete(run_id)` | `db.runs.delete(&run)` | ✅ OK |

### 2.3 Run State Diagram

```
                    ┌──────────────────────────────────────┐
                    │                                      │
                    ▼                                      │
    ┌────────┐  ┌────────┐  ┌───────────┐  ┌──────────┐  │
    │ Active │─▶│ Paused │─▶│ Completed │─▶│ Archived │  │
    └────┬───┘  └────┬───┘  └───────────┘  └──────────┘  │
         │           │                          ▲         │
         │           │      ┌────────┐          │         │
         ├───────────┴─────▶│ Failed │──────────┤         │
         │                  └────────┘          │         │
         │                                      │         │
         │                  ┌───────────┐       │         │
         └─────────────────▶│ Cancelled │───────┴─────────┘
                            └───────────┘
```

### 2.4 Run Metadata

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Update metadata | `run_update_metadata(run, meta)` | `db.runs.update_metadata(&run, meta)` | ✅ OK |
| Add tags | `run_add_tags(run, tags)` | `db.runs.add_tags(&run, tags)` | ✅ OK |
| Remove tags | `run_remove_tags(run, tags)` | `db.runs.remove_tags(&run, tags)` | ✅ OK |
| Get tags | `run_get_tags(run)` | `db.runs.get_tags(&run)` | ✅ OK |
| Query by tag | `run_query_by_tag(tag)` | `db.runs.query_by_tag(tag)` | ✅ OK |
| Query by status | `run_query_by_status(state)` | `db.runs.query_by_status(state)` | ✅ OK |
| Search runs | `run_search(query, limit)` | `db.runs.search(query, limit)` | ✅ OK |
| Count runs | `run_count(status)` | `db.runs.count(status)` | ✅ OK |

### 2.5 Run Hierarchy

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create child run | `run_create_child(parent, meta)` | `db.runs.create_child(&parent, meta)` | ✅ OK |
| Get children | `run_get_children(parent)` | `db.runs.children(&parent)` | ✅ OK |
| Get parent | `run_get_parent(run)` | `db.runs.parent(&run)` | ✅ OK |

### 2.6 Run Retention

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set retention policy | `run_set_retention(run, policy)` | `db.runs.set_retention(&run, policy)` | ✅ OK |
| Get retention policy | `run_get_retention(run)` | `db.runs.get_retention(&run)` | ✅ OK |

---

## 3. Run Bundling (Export/Import)

### 3.1 Export

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Export run to file | `export_run(run_id, path)` | `db.runs.export(&run, path)` | ✅ OK |
| Export with options | `export_run_with_options(run, path, opts)` | `db.runs.export_with(&run, path, opts)` | ✅ OK |

**Export Options:**
- Include/exclude metadata
- Custom compression level
- Verify after export

### 3.2 Import

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Import run from file | `import_run(path)` | `db.runs.import(path)` | ✅ OK |
| Verify bundle | `verify_bundle(path)` | `db.runs.verify_bundle(path)` | ✅ OK |

**Bundle Format:** `.runbundle.tar.zst`
```
<run_id>.runbundle.tar.zst
└── runbundle/
    ├── MANIFEST.json    # Format version, checksums
    ├── RUN.json         # Run metadata
    └── WAL.runlog       # Run-scoped WAL entries
```

**Export Constraints:**
- Only terminal runs can be exported (Completed, Failed, Cancelled, Archived)
- Export is deterministic (same run = same bundle)
- Bundles are immutable artifacts

---

## 4. Transactions

### 4.1 Implicit Transactions

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Execute in transaction | `db.transaction(run, \|tx\| {...})` | `db.transaction(&run, \|tx\| {...})` | ✅ OK |
| Transaction with version | `db.transaction_with_version(...)` | `db.transaction_versioned(...)` | ✅ Rename |
| Transaction with timeout | `db.transaction_with_timeout(...)` | `db.transaction_timeout(...)` | ✅ Rename |
| Transaction with retry | `db.transaction_with_retry(...)` | `db.transaction_retry(...)` | ✅ Rename |

### 4.2 Retry Configuration

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Default retry | `RetryConfig::new()` | `RetryConfig::new()` | ✅ OK |
| No retry | `RetryConfig::no_retry()` | `RetryConfig::none()` | ✅ Rename |
| Max retries | `.with_max_retries(n)` | `.max_retries(n)` | ✅ Simplify |
| Base delay | `.with_base_delay_ms(ms)` | `.base_delay_ms(ms)` | ✅ Simplify |
| Max delay | `.with_max_delay_ms(ms)` | `.max_delay_ms(ms)` | ✅ Simplify |

### 4.3 Explicit Transaction Control

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Begin transaction | `txn_begin(options)` | `db.begin(&run)` | ✅ Simplify |
| Commit | `txn_commit()` | `tx.commit()` | ✅ OK |
| Rollback | `txn_rollback()` | `tx.rollback()` | ✅ OK |
| Transaction info | `txn_info()` | `tx.info()` | ✅ OK |
| Is active | `txn_is_active()` | `tx.is_active()` | ✅ OK |

### 4.4 Savepoints

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create savepoint | `savepoint(name)` | `tx.savepoint(name)` | ✅ OK |
| Rollback to savepoint | `rollback_to(name)` | `tx.rollback_to(name)` | ✅ OK |
| Release savepoint | `release_savepoint(name)` | `tx.release(name)` | ✅ Simplify |

### 4.5 Transaction Metrics

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get metrics | `db.metrics()` | `db.transaction_metrics()` | ✅ Rename |

---

## 5. Key-Value Store

### 5.1 Basic Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set value | `kv_put(run, key, value)` | `db.kv.set(key, value)` | ✅ Rename |
| Set in run | - | `db.kv.set_in(&run, key, value)` | ✅ Add |
| Put (returns version) | `kv_put(run, key, value)` | `db.kv.put(&run, key, value)` | ✅ OK |
| Get value | `get(key)` / `kv_get(run, key)` | `db.kv.get(key)` | ✅ OK |
| Get versioned | `getv(key)` | `db.kv.get_versioned(key)` | ✅ Rename |
| Get in run | `kv_get(run, key)` | `db.kv.get_in(&run, key)` | ✅ OK |
| Delete | `del(key)` / `kv_delete(run, key)` | `db.kv.delete(key)` | ✅ OK |
| Exists | `exists(key)` / `kv_exists(run, key)` | `db.kv.exists(key)` | ✅ OK |

### 5.2 Atomic Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Increment by 1 | `incr(key)` | `db.kv.incr(key)` | ✅ OK |
| Increment by delta | `incrby(key, delta)` | `db.kv.incr_by(key, delta)` | ✅ OK |
| Decrement by 1 | `decr(key)` | `db.kv.decr(key)` | ✅ OK |
| Decrement by delta | `decrby(key, delta)` | `db.kv.decr_by(key, delta)` | ✅ OK |
| Set if not exists | `setnx(key, value)` | `db.kv.set_nx(key, value)` | ✅ OK |
| Get and set | `getset(key, value)` | `db.kv.get_set(key, value)` | ✅ OK |
| Get and delete | (missing) | `db.kv.get_del(key)` | ❌ Add |

### 5.3 Compare-and-Swap

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| CAS by version | `kv_cas_version(run, key, ver, val)` | `db.kv.cas(&run, key, ver, val)` | ✅ OK |
| CAS by value | `kv_cas_value(run, key, old, new)` | `db.kv.cas_value(&run, key, old, new)` | ✅ OK |

### 5.4 Batch Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get multiple | `mget(keys)` | `db.kv.mget(keys)` | ✅ OK |
| Set multiple | `mset(entries)` | `db.kv.mset(entries)` | ✅ OK |
| Delete multiple | `mdel(keys)` | `db.kv.mdelete(keys)` | ✅ OK |
| Exists multiple | `mexists(keys)` | `db.kv.mexists(keys)` | ✅ OK |
| Put multiple (version) | `kv_mput(run, entries)` | `db.kv.mput(&run, entries)` | ✅ OK |

### 5.5 Scanning & History

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| List keys | `kv_keys(run, prefix, limit)` | `db.kv.keys(prefix)` | ✅ OK |
| Scan with cursor | `kv_scan(run, prefix, limit, cursor)` | `db.kv.scan(prefix, cursor)` | ✅ OK |
| Get history | `kv_history(run, key, limit, before)` | `db.kv.history(key, limit)` | ✅ OK |
| Get at version | `kv_get_at(run, key, version)` | `db.kv.get_at(&run, key, version)` | ✅ OK |

### 5.6 Missing KV Operations

| Capability | Proposed API | Priority |
|------------|--------------|----------|
| Count all keys | `db.kv.count()` | P1 |
| Rename key | `db.kv.rename(old, new)` | P2 |
| Get type | `db.kv.type_of(key)` | P2 |
| Update function | `db.kv.update(key, f)` | P2 |
| Get or default | `db.kv.get_or(key, default)` | P2 |

---

## 6. JSON Documents

### 6.1 Document Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set document | `json_set(run, key, "$", doc)` | `db.json.set(key, doc)` | ✅ OK |
| Get document | `json_get(run, key, "$")` | `db.json.get(key)` | ✅ OK |
| Delete document | `json_delete(run, key, "$")` | `db.json.delete(key)` | ✅ OK |
| Document exists | `json_exists(run, key)` | `db.json.exists(key)` | ✅ OK |

### 6.2 Path Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set at path | `json_set(run, key, path, val)` | `db.json.set_path(key, path, val)` | ✅ OK |
| Get at path | `json_get(run, key, path)` | `db.json.get_path(key, path)` | ✅ OK |
| Delete at path | `json_delete(run, key, path)` | `db.json.delete_path(key, path)` | ✅ OK |
| Merge at path | `json_merge(run, key, path, patch)` | `db.json.merge(key, path, patch)` | ✅ OK |

### 6.3 Array Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Push to array | `json_array_push(run, key, path, vals)` | `db.json.push(key, path, vals)` | ✅ OK |
| Pop from array | `json_array_pop(run, key, path)` | `db.json.pop(key, path)` | ✅ OK |
| Array length | `json_arrlen(key, path)` | `db.json.array_len(key, path)` | ✅ OK |

### 6.4 Numeric Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Increment number | `json_increment(run, key, path, delta)` | `db.json.incr(key, path, delta)` | ✅ OK |

### 6.5 Type Inspection

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get type at path | `json_type(key, path)` | `db.json.type_of(key, path)` | ✅ OK |
| Get object keys | `json_objkeys(key, path)` | `db.json.obj_keys(key, path)` | ✅ OK |
| Get object length | `json_objlen(key, path)` | `db.json.obj_len(key, path)` | ✅ OK |
| String length | `json_strlen(key, path)` | `db.json.str_len(key, path)` | ✅ OK |

### 6.6 Querying & Search

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Query by field | `json_query(run, path, value, limit)` | `db.json.query(path, value, limit)` | ✅ OK |
| Full-text search | `json_search(run, query, k)` | `db.json.search(query, limit)` | ✅ OK |
| List documents | `json_list(run, prefix, cursor, limit)` | `db.json.list(prefix, limit)` | ✅ OK |
| Count documents | `json_count(run)` | `db.json.count()` | ✅ OK |

### 6.7 Batch & History

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get multiple | `json_batch_get(run, keys)` | `db.json.mget(keys)` | ✅ OK |
| Create multiple | `json_batch_create(run, docs)` | `db.json.mcreate(docs)` | ✅ OK |
| Document history | `json_history(run, key, limit, before)` | `db.json.history(key, limit)` | ✅ OK |
| Get version | `json_get_version(run, key)` | `db.json.version(key)` | ✅ OK |

---

## 7. Event Streams

### 7.1 Appending

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Append event | `event_append(run, stream, payload)` | `db.events.append(stream, payload)` | ✅ OK |
| Batch append | `event_append_batch(run, events)` | `db.events.append_batch(stream, payloads)` | ✅ OK |

### 7.2 Reading

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get by sequence | `event_get(run, stream, seq)` | `db.events.get(stream, seq)` | ✅ OK |
| Get range | `event_range(run, stream, start, end, limit)` | `db.events.range(stream, start, end)` | ✅ OK |
| Get range limited | - | `db.events.range_limit(stream, start, end, limit)` | ✅ OK |
| Reverse range | `event_rev_range(...)` | `db.events.range_rev(stream, start, end)` | ✅ OK |
| Get head | `event_head(run, stream)` | `db.events.head(stream)` | ✅ OK |

### 7.3 Stream Info

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Stream length | `event_len(run, stream)` | `db.events.len(stream)` | ✅ OK |
| Latest sequence | `event_latest_sequence(run, stream)` | `db.events.latest(stream)` | ✅ OK |
| Stream info | `event_stream_info(run, stream)` | `db.events.info(stream)` | ✅ OK |
| List streams | `event_streams(run)` | `db.events.streams()` | ✅ OK |

### 7.4 Verification

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Verify chain | `event_verify_chain(run)` | `db.events.verify_chain()` | ✅ OK |

### 7.5 Missing Event Operations

| Capability | Proposed API | Priority |
|------------|--------------|----------|
| Delete stream | `db.events.delete_stream(stream)` | P2 |
| Trim stream | `db.events.trim(stream, max_len)` | P2 |
| Subscribe | `db.events.subscribe(stream, callback)` | P3 |

---

## 8. State Cells

### 8.1 Basic Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set cell | `state_set(run, cell, value)` | `db.state.set(cell, value)` | ✅ OK |
| Get cell | `state_get(run, cell)` | `db.state.get(cell)` | ✅ OK |
| Delete cell | `state_delete(run, cell)` | `db.state.delete(cell)` | ✅ OK |
| Cell exists | `state_exists(run, cell)` | `db.state.exists(cell)` | ✅ OK |

### 8.2 Compare-and-Swap

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| CAS | `state_cas(run, cell, counter, value)` | `db.state.cas(cell, counter, value)` | ✅ OK |
| Initialize | `state_init(run, cell, value)` | `db.state.init(cell, value)` | ✅ OK |
| Get or init | `state_get_or_init(run, cell, default)` | `db.state.get_or_init(cell, default)` | ✅ OK |

### 8.3 Transitions

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Transition | `state_transition(run, cell, f)` | `db.state.transition(cell, f)` | ✅ OK |
| Transition or init | `state_transition_or_init(run, cell, init, f)` | `db.state.transition_or_init(cell, init, f)` | ✅ OK |

### 8.4 Listing & History

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| List cells | `state_list(run)` | `db.state.list()` | ✅ OK |
| Cell history | `state_history(run, cell, limit, before)` | `db.state.history(cell, limit)` | ✅ OK |

---

## 9. Vector Store

### 9.1 Vector CRUD

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Upsert vector | `vector_upsert(run, coll, key, vec, meta)` | `db.vectors.upsert(coll, key, vec, meta)` | ✅ OK |
| Upsert with source | `vector_upsert_with_source(...)` | `db.vectors.upsert_with_source(...)` | ✅ OK |
| Get vector | `vector_get(run, coll, key)` | `db.vectors.get(coll, key)` | ✅ OK |
| Delete vector | `vector_delete(run, coll, key)` | `db.vectors.delete(coll, key)` | ✅ OK |
| Vector exists | (missing) | `db.vectors.exists(coll, key)` | ❌ Add |

### 9.2 Search

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Similarity search | `vector_search(run, coll, query, k, filter, metric)` | `db.vectors.search(coll, query, k)` | ✅ OK |
| Search with filter | - | `db.vectors.search_filter(coll, query, k, filter)` | ✅ OK |
| Search with options | - | `db.vectors.search_with(coll, query, k, opts)` | ✅ OK |
| Search with budget | `vector_search_with_budget(...)` | `db.vectors.search_budget(coll, query, k, budget)` | ✅ OK |

### 9.3 Collection Management

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create collection | `vector_create_collection(run, coll, dim, metric)` | `db.vectors.create_collection(coll, dim, metric)` | ✅ OK |
| Drop collection | `vector_drop_collection(run, coll)` | `db.vectors.drop_collection(coll)` | ✅ OK |
| List collections | `vector_list_collections(run)` | `db.vectors.collections()` | ✅ OK |
| Collection info | `vector_collection_info(run, coll)` | `db.vectors.collection_info(coll)` | ✅ OK |
| Collection exists | `vector_collection_exists(run, coll)` | `db.vectors.collection_exists(coll)` | ✅ OK |
| Vector count | `vector_count(run, coll)` | `db.vectors.count(coll)` | ✅ OK |

### 9.4 Batch Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Batch upsert | `vector_upsert_batch(run, coll, vectors)` | `db.vectors.mupsert(coll, vectors)` | ✅ OK |
| Batch get | `vector_get_batch(run, coll, keys)` | `db.vectors.mget(coll, keys)` | ✅ OK |
| Batch delete | `vector_delete_batch(run, coll, keys)` | `db.vectors.mdelete(coll, keys)` | ✅ OK |

### 9.5 Scanning & History

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| List keys | `vector_list_keys(run, coll, limit, cursor)` | `db.vectors.keys(coll, limit)` | ✅ OK |
| Scan vectors | `vector_scan(run, coll, limit, cursor)` | `db.vectors.scan(coll, cursor)` | ✅ OK |
| Vector history | `vector_history(run, coll, key, limit, before)` | `db.vectors.history(coll, key, limit)` | ✅ OK |
| Get at version | `vector_get_at(run, coll, key, version)` | `db.vectors.get_at(coll, key, version)` | ✅ OK |

---

## 10. Replay & Recovery

### 10.1 Read-Only View

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Replay run | `db.replay_run(run_id)` | `db.runs.replay(&run)` | ✅ OK |
| Get KV in view | `view.get_kv(key)` | `view.kv.get(key)` | ✅ OK |
| Apply operations | `view.apply_kv_put(...)` | (internal) | N/A |
| Get operation count | `view.operation_count()` | `view.operation_count()` | ✅ OK |

### 10.2 Run Diff

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Diff two runs | `db.diff_runs(run_a, run_b)` | `db.runs.diff(&run_a, &run_b)` | ✅ OK |
| Get added keys | `diff.added` | `diff.added()` | ✅ OK |
| Get removed keys | `diff.removed` | `diff.removed()` | ✅ OK |
| Get modified keys | `diff.modified` | `diff.modified()` | ✅ OK |
| Get summary | `diff.summary()` | `diff.summary()` | ✅ OK |

---

## 11. Retention Policies

### 11.1 Policy Management

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get retention | `retention_get(run)` | `db.retention.get(&run)` | ✅ OK |
| Set retention | `retention_set(run, policy)` | `db.retention.set(&run, policy)` | ✅ OK |
| Clear retention | `retention_clear(run)` | `db.retention.clear(&run)` | ✅ OK |

### 11.2 Retention Policies

| Policy | Constructor | Behavior |
|--------|-------------|----------|
| Keep all | `RetentionPolicy::KeepAll` | Never delete history |
| Keep last N | `RetentionPolicy::KeepLast(n)` | Keep N versions |
| Keep for duration | `RetentionPolicy::KeepFor(dur)` | Keep within time window |
| Composite | `RetentionPolicy::Composite(vec)` | Union of policies |

### 11.3 Garbage Collection

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get GC stats | `retention_stats(run)` | `db.retention.stats(&run)` | ✅ OK |
| Trigger GC | `retention_gc(run)` | `db.retention.gc(&run)` | ✅ OK |

---

## 12. Search

### 12.1 Hybrid Search

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get search interface | `db.hybrid()` | `db.search` | ✅ OK |
| Execute search | `hybrid.search(request)` | `db.search.query(request)` | ✅ OK |

### 12.2 Search Request Options

| Option | Description |
|--------|-------------|
| Text query | Full-text search string |
| Vector query | Similarity search vector |
| Filters | Metadata filters |
| Limit | Max results |
| Primitives | Which primitives to search |

---

## 13. System Capabilities

### 13.1 Capability Discovery

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Get capabilities | `capabilities()` | `db.capabilities()` | ✅ OK |
| Get limits | `capabilities.limits` | `db.limits()` | ✅ OK |

### 13.2 System Limits

| Limit | Description |
|-------|-------------|
| `max_key_bytes` | Maximum key size |
| `max_string_bytes` | Maximum string value size |
| `max_bytes_len` | Maximum bytes value size |
| `max_value_bytes_encoded` | Maximum encoded value size |
| `max_array_len` | Maximum array length |
| `max_object_entries` | Maximum object entries |
| `max_nesting_depth` | Maximum JSON nesting |
| `max_vector_dim` | Maximum vector dimension |

---

---

## 14. Versioning & History Access

### 14.1 Version Types

Every write in Strata produces a version. Different primitives use different version schemes:

| Primitive | Version Type | Behavior |
|-----------|--------------|----------|
| KV | `TxnId` | Global transaction ID, monotonic |
| JSON | `TxnId` | Global transaction ID, monotonic |
| Events | `Sequence` | Per-run sequence number, append-only |
| State | `Counter` | Per-cell counter, increments on each write |
| Vectors | `TxnId` | Global transaction ID, monotonic |
| Runs | `TxnId` | Global transaction ID for metadata changes |

### 14.2 Reading Current Version

| Primitive | Current API | Proposed API | Status |
|-----------|-------------|--------------|--------|
| KV | `kv_get(run, key)` → `Versioned<Value>` | `db.kv.get_versioned(key)` | ✅ OK |
| JSON | `json_get(run, key, path)` → `Versioned<Value>` | `db.json.get_versioned(key)` | ✅ OK |
| Events | `event_get(run, stream, seq)` → `Versioned<Value>` | `db.events.get_versioned(stream, seq)` | ✅ OK |
| State | `state_get(run, cell)` → `Versioned<Value>` | `db.state.get_versioned(cell)` | ✅ OK |
| Vectors | `vector_get(run, coll, key)` → `Versioned<VectorData>` | `db.vectors.get_versioned(coll, key)` | ✅ OK |

### 14.3 Reading at Specific Version (Point-in-Time)

| Primitive | Current API | Proposed API | Status |
|-----------|-------------|--------------|--------|
| KV | `kv_get_at(run, key, version)` | `db.kv.get_at(&run, key, version)` | ✅ OK |
| JSON | (missing) | `db.json.get_at(&run, key, version)` | ❌ **Add** |
| Events | `event_get(run, stream, seq)` | `db.events.get(stream, seq)` | ✅ OK (by sequence) |
| State | (missing) | `db.state.get_at(&run, cell, version)` | ❌ **Add** |
| Vectors | `vector_get_at(run, coll, key, ver)` | `db.vectors.get_at(coll, key, version)` | ✅ OK |

### 14.4 Reading Version History

| Primitive | Current API | Proposed API | Status |
|-----------|-------------|--------------|--------|
| KV | `kv_history(run, key, limit, before)` | `db.kv.history(key, limit)` | ✅ OK |
| JSON | `json_history(run, key, limit, before)` | `db.json.history(key, limit)` | ✅ OK |
| Events | `event_range(run, stream, start, end)` | `db.events.range(stream, start, end)` | ✅ OK |
| State | `state_history(run, cell, limit, before)` | `db.state.history(cell, limit)` | ✅ OK |
| Vectors | `vector_history(run, coll, key, limit)` | `db.vectors.history(coll, key, limit)` | ✅ OK |
| Runs | (missing) | `db.runs.history(&run, limit)` | ❌ **Add** |

### 14.5 Version Pagination

All history methods should support consistent pagination:

```rust
// Proposed unified history API
pub struct HistoryPage<T> {
    pub items: Vec<Versioned<T>>,
    pub next_cursor: Option<Version>,  // Pass to next call
    pub has_more: bool,
}

// Usage
let page1 = db.kv.history("key", 10)?;           // First 10 versions
let page2 = db.kv.history_after("key", 10, page1.next_cursor)?;  // Next 10
```

### 14.6 History Access Summary

| Capability | Proposed API | Notes |
|------------|--------------|-------|
| Get with version | `get_versioned(key)` | Returns `Versioned<T>` |
| Get at version | `get_at(&run, key, version)` | Point-in-time read |
| Get history | `history(key, limit)` | Most recent first |
| Get history after | `history_after(key, limit, cursor)` | Paginated |
| Get history before | `history_before(key, limit, cursor)` | Paginated |
| Get first version | `first_version(key)` | Oldest version |
| Get current version | `current_version(key)` | Latest version number only |

---

## 15. Snapshots & Checkpoints

### 15.1 Snapshot Operations

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create snapshot | `storage.create_snapshot()` | `db.snapshot()` | ⚠️ Internal only |
| Read from snapshot | (internal) | `snapshot.kv.get(key)` | ⚠️ Internal only |

**Note**: Snapshots are currently internal for MVCC. Should we expose?

### 15.2 Proposed Checkpoint API

Checkpoints are named, persistent snapshots that survive restart:

```rust
// Create checkpoint
db.checkpoint("before-migration")?;

// List checkpoints
let checkpoints = db.checkpoints()?;
// Returns: [CheckpointInfo { name, created_at, size_bytes }]

// Read from checkpoint
let view = db.at_checkpoint("before-migration")?;
let value = view.kv.get("key")?;

// Delete checkpoint
db.delete_checkpoint("before-migration")?;
```

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Create checkpoint | (missing) | `db.checkpoint(name)` | ❌ **Add** |
| List checkpoints | (missing) | `db.checkpoints()` | ❌ **Add** |
| Read from checkpoint | (missing) | `db.at_checkpoint(name)` | ❌ **Add** |
| Delete checkpoint | (missing) | `db.delete_checkpoint(name)` | ❌ **Add** |
| Checkpoint info | (missing) | `db.checkpoint_info(name)` | ❌ **Add** |

### 15.3 Checkpoint vs Run Export

| Feature | Checkpoint | Run Export |
|---------|------------|------------|
| Scope | Entire database | Single run |
| Portability | Same database only | Any database |
| Format | Internal | `.runbundle.tar.zst` |
| Use case | Rollback, testing | Sharing, archival |

---

## 16. Compaction & Storage Management

### 16.1 Current Storage Layer

The LSM-tree storage automatically compacts in the background. Users have limited control:

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Force flush | `db.flush()` | `db.flush()` | ✅ OK |
| Storage stats | (missing) | `db.storage_stats()` | ❌ **Add** |
| Trigger compaction | (missing) | `db.compact()` | ❌ **Add** (P3) |

### 16.2 Proposed Storage Stats

```rust
pub struct StorageStats {
    pub total_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub data_size_bytes: u64,
    pub index_size_bytes: u64,
    pub version_count: u64,           // Total versions across all keys
    pub tombstone_count: u64,         // Pending deletes
    pub pending_compaction_bytes: u64,
}

let stats = db.storage_stats()?;
```

### 16.3 Run-Level Storage

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Run size | (missing) | `db.runs.size(&run)` | ❌ **Add** |
| Run version count | (missing) | `db.runs.version_count(&run)` | ❌ **Add** |
| Run key count | (missing) | `db.runs.key_count(&run)` | ❌ **Add** |

---

## 17. Retention Policies (Detailed)

### 17.1 Policy Types

| Policy | Constructor | Behavior |
|--------|-------------|----------|
| Keep All | `RetentionPolicy::KeepAll` | Never delete any version |
| Keep Last N | `RetentionPolicy::KeepLast(n)` | Keep N most recent versions per key |
| Keep Duration | `RetentionPolicy::KeepFor(dur)` | Keep versions within time window |
| Composite | `RetentionPolicy::Composite(vec)` | Union of policies (most permissive) |

### 17.2 Retention Scope

| Scope | Current | Proposed | Status |
|-------|---------|----------|--------|
| Per-run | ✅ `run_set_retention(run, policy)` | `db.runs.set_retention(&run, policy)` | ✅ OK |
| Per-primitive | ❌ Not supported | `db.kv.set_retention(&run, policy)` | ❌ **Consider** |
| Per-key | ❌ Not supported | `db.kv.set_key_retention(&run, key, policy)` | ❌ **Consider** |
| Global default | ❌ Not supported | `db.set_default_retention(policy)` | ❌ **Add** |

### 17.3 Retention API

| Capability | Current API | Proposed API | Status |
|------------|-------------|--------------|--------|
| Set run retention | `run_set_retention(run, policy)` | `db.runs.set_retention(&run, policy)` | ✅ OK |
| Get run retention | `run_get_retention(run)` | `db.runs.get_retention(&run)` | ✅ OK |
| Clear retention | `retention_clear(run)` | `db.retention.clear(&run)` | ✅ OK |
| Get GC stats | `retention_stats(run)` | `db.retention.stats(&run)` | ✅ OK |
| Trigger GC | `retention_gc(run)` | `db.retention.gc(&run)` | ✅ OK |
| Set global default | (missing) | `db.set_default_retention(policy)` | ❌ **Add** |

### 17.4 Retention Examples

```rust
// Keep last 10 versions per key
db.runs.set_retention(&run, RetentionPolicy::KeepLast(10))?;

// Keep versions from last 7 days
db.runs.set_retention(&run, RetentionPolicy::KeepFor(Duration::days(7)))?;

// Keep last 5 OR last 24 hours (whichever keeps more)
db.runs.set_retention(&run, RetentionPolicy::Composite(vec![
    RetentionPolicy::KeepLast(5),
    RetentionPolicy::KeepFor(Duration::hours(24)),
]))?;

// Check what would be garbage collected
let stats = db.retention.stats(&run)?;
println!("Eligible for GC: {} versions, {} bytes",
    stats.gc_eligible_versions,
    stats.estimated_reclaimable_bytes);

// Trigger immediate GC
db.retention.gc(&run)?;
```

### 17.5 Retention Behavior

| Aspect | Behavior |
|--------|----------|
| When applied | Background thread + on write |
| What's deleted | Old versions, not current value |
| Tombstones | Kept until all versions removed |
| Events | Sequence gaps allowed after GC |
| State cells | Counter continues, old values deleted |

---

## 18. Version Query Patterns

### 18.1 Common Patterns

```rust
// Pattern 1: Get current value with version
let versioned = db.kv.get_versioned("key")?;
if let Some(v) = versioned {
    println!("Value: {:?}, Version: {}", v.value(), v.version());
}

// Pattern 2: Get value at specific point in time
let old_value = db.kv.get_at(&run, "key", Version::txn(42))?;

// Pattern 3: Get last N versions
let history = db.kv.history("key", 10)?;
for v in history {
    println!("{}: {:?}", v.version(), v.value());
}

// Pattern 4: Compare-and-swap with version
let current = db.kv.get_versioned("counter")?;
if let Some(v) = current {
    let new_value = v.value().as_i64().unwrap() + 1;
    db.kv.cas(&run, "counter", v.version(), Value::Int(new_value))?;
}

// Pattern 5: Replay from specific version
let view = db.runs.replay(&run)?;
// view contains state as of run completion
```

### 18.2 Proposed Convenience Methods

| Method | Description | Status |
|--------|-------------|--------|
| `get_or(key, default)` | Get current or return default | ❌ **Add** |
| `get_prev(key)` | Get previous version | ❌ **Add** |
| `get_at_time(key, timestamp)` | Get value at timestamp | ❌ **Add** |
| `versions(key)` | Get all version numbers only | ❌ **Add** |
| `diff(key, v1, v2)` | Diff two versions | ❌ **Add** |

---

## 19. Missing Capabilities Summary

### 19.1 Versioning & History Gaps

| Primitive | `get_at` | `history` | `first_version` | `current_version` |
|-----------|----------|-----------|-----------------|-------------------|
| KV | ✅ | ✅ | ❌ | ❌ |
| JSON | ❌ | ✅ | ❌ | ❌ |
| Events | ✅ (by seq) | ✅ (range) | ❌ | ❌ |
| State | ❌ | ✅ | ❌ | ❌ |
| Vectors | ✅ | ✅ | ❌ | ❌ |
| Runs | N/A | ❌ | ❌ | ❌ |

### 19.2 Storage Management Gaps

| Capability | Status | Priority |
|------------|--------|----------|
| `db.checkpoint(name)` | ❌ Missing | P2 |
| `db.at_checkpoint(name)` | ❌ Missing | P2 |
| `db.storage_stats()` | ❌ Missing | P1 |
| `db.runs.size(&run)` | ❌ Missing | P1 |
| `db.compact()` | ❌ Missing | P3 |
| Global default retention | ❌ Missing | P2 |
| Per-key retention | ❌ Missing | P3 |

---

## Summary: Changes Needed

### Naming Cleanup (Apply UNIFIED_API_DESIGN.md)

| Category | Count |
|----------|-------|
| Remove method prefixes | 50+ |
| Rename to simpler names | 20+ |
| Standardize suffixes | 15+ |

### Missing Capabilities to Add

| Priority | Capability |
|----------|------------|
| P1 | `db.kv.count()` - Count all keys |
| P1 | `db.info()` - Database info |
| P1 | `db.stats()` - Storage stats |
| P2 | `db.kv.get_del()` - Get and delete |
| P2 | `db.kv.rename()` - Rename key |
| P2 | `db.events.delete_stream()` - Delete stream |
| P2 | `db.events.trim()` - Trim stream |
| P2 | `db.vectors.exists()` - Check vector exists |
| P3 | `db.events.subscribe()` - Subscribe to stream |
| P3 | Watch/observe capabilities |

### Structural Changes

1. **Unify API layers** - Merge Facade and Substrate into single API
2. **Create `strata` crate** - Single entry point
3. **Primitive accessors** - `db.kv`, `db.json`, etc.
4. **Consistent pagination** - Standard `Page<T>` return type
5. **Single error type** - Consolidate `Error` and `StrataError`
