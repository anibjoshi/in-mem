# RunBundle: Portable Execution Artifacts

> **Status**: Post-MVP (Architectural Intent)
> **Scope**: Offline execution portability
> **Non-Goals**: Replication, sync, cloud orchestration
> **Core Principle**: A run is an immutable execution commit

---

## 1. Purpose

A **RunBundle** is a **portable, immutable artifact** representing a **single completed Strata run**.

Its only purpose is to allow a finished execution to be:

* exported from one Strata instance
* stored externally (filesystem, object store, VCS, etc.)
* imported into another Strata instance
* replayed, inspected, or diffed deterministically

RunBundle is **not**:

* a replication mechanism
* a synchronization protocol
* a storage backend
* a distributed execution system

It is an **execution artifact**, nothing more.

---

## 2. MVP Architectural Restraint

This design explicitly avoids:

* live WAL tailing
* partial or streaming export
* incremental sync
* background upload or download
* conflict detection or resolution
* multi-node coordination
* cloud authentication

All RunBundle behavior is:

* explicit
* offline
* post-execution
* deterministic

Nothing happens automatically.

---

## 3. Conceptual Model

A RunBundle corresponds to **exactly one closed run**.

```
┌───────────────────────────────┐
│ Strata Instance               │
│                               │
│ begin_run(run_id)             │
│   transactions execute        │
│ end_run(run_id)               │
│                               │
│ export_run(run_id) ──────┐    │
└───────────────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │ RunBundle               │
                    │ (immutable artifact)   │
                    └────────────────────────┘
```

A RunBundle may only be created after:

* the run is closed
* all transactions are committed
* no further mutations are possible

---

## 4. What a RunBundle Contains

A RunBundle captures **everything required to deterministically reconstruct a run**.

Conceptually, it includes:

### Required

* Run identity and metadata

  * run_id
  * creation and close timestamps
  * Strata format version
* Run-scoped write history

  * all WAL entries belonging to the run
  * ordered and checksummed
  * no entries from other runs

### Optional (non-required)

* Snapshot at run boundary
* Index hints for faster replay

A RunBundle does **not** contain:

* global database state
* other runs
* mutable or live data
* background processes

---

## 5. Deterministic Replay Contract

A RunBundle must satisfy the following invariant:

> Replaying a RunBundle into an empty Strata instance produces the same logical state as the original run.

This includes:

* KV contents
* JSON documents
* EventLog entries
* Vector data
* RunIndex metadata

Correctness is mandatory.
Performance is secondary.

---

## 6. Export Semantics

Export is an **explicit, read-only operation**.

Conceptually:

```
export_run(run_id) → RunBundle
```

Rules:

* Only closed runs may be exported
* Export does not modify the source database
* Export produces a stable, immutable artifact
* Export has no side effects

---

## 7. Import Semantics

Import is also explicit:

```
import_run(run_bundle) → ImportedRunInfo
```

Rules:

* Import does not require shared history
* Imported runs are immutable
* A new local run_id may be assigned
* Imported runs are isolated from local runs

No attempt is made to:

* merge runs
* deduplicate state
* reconcile conflicts

Imported runs coexist. They never overwrite.

---

## 8. Relationship to Existing Persistence

RunBundle does **not** replace:

* WAL
* snapshots
* checkpoints
* compaction
* retention policies

It is a **projection** of existing persistence artifacts.

The storage engine remains unchanged.

RunBundle is a **logical boundary**, not a new storage layer.

---

## 9. Explicit Non-Goals

RunBundle explicitly does not provide:

* live synchronization
* incremental export
* background upload
* cloud authentication
* encryption
* replication
* sharding
* remote querying

Those concerns belong to systems built *on top of* RunBundles, not inside Strata.

---

## 10. Why This Exists

RunBundle exists to enable:

* reproducible execution history
* offline transfer of runs
* deterministic debugging
* future push-based cloud versioning
* safe experimentation without data loss

All without compromising:

* correctness
* determinism
* simplicity
* testability

---

## 11. MVP Posture

For MVP:

* RunBundle is conceptual only
* No storage redesign is required
* No distributed systems are introduced
* No automation is added

The only commitment is:

> Runs are immutable, exportable execution artifacts.

Everything else is deferred.

---

## 12. API Placement Decision

### Export should **not** live on RunIndex

RunIndex is:

* metadata
* lifecycle
* indexing

Export requires:

* WAL slicing
* snapshot boundaries
* write materialization
* storage coordination

That is **database orchestration**, not indexing.

### Correct MVP shape

Expose export at the database level:

```text
db.export_run(run_id, path) → RunExportInfo
db.import_run(path) → ImportedRunInfo
```

Internally, export will:

* consult RunIndex
* traverse WAL
* validate run closure

But none of that leaks into RunIndex’s API.

### Optional RunIndex support

RunIndex may expose **capability queries**, not export:

```text
runindex.is_closed(run_id) → bool
runindex.run_bounds(run_id) → RunBounds
```

This keeps RunIndex:

* small
* predictable
* pure

---

## 13. Summary

RunBundle is the natural extension of Strata’s execution-first model.

It treats runs as:

* commits, not logs
* artifacts, not state
* history, not replication

By exercising architectural restraint now, RunBundle remains:

* simple
* correct
* inevitable

When it arrives, it will not feel like a new feature.

It will feel like something Strata was always meant to do.
