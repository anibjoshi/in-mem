# Agentic In-Memory Database

## Core Vision

This project is an attempt to design a new kind of database for the agentic AI era.

Not a faster Postgres.
Not a Redis replacement.
Not a vector database with features bolted on.

This system is meant to be the *native memory substrate for AI agents*.

Agents do not just store rows. They reason, branch, coordinate, speculate, call tools, revise plans, and learn from traces of their own behavior. Their memory is not just data; it is *process, provenance, intent, and causality*.

The core thesis is:

> Agents need a database that treats runs, events, decisions, coordination, and replay as first-class—not as logs bolted onto a relational system.

This database is designed to be:

* In-memory and ultra-low latency
* Deterministic and replayable
* Concurrency-safe for multi-agent systems
* Traceable and auditable
* Native to agent workflows

It is both:

**A) An Agent Memory Database**
**C) A Deterministic Replay + Debugging Engine**

These two are inseparable. Memory without replay is untrustworthy. Replay without structured memory is useless.

---

## The Five Core Primitives

The system is built around five primitives that cover nearly all agent memory needs.

### 1. KV / Document Store

Purpose: Fast, general-purpose state storage.

* String keys
* JSON-compatible values
* Optional schema enforcement
* Versioned
* Namespaced
* TTL support

This is the default "working memory" for agents.

Typical use:

* Scratchpads
* Partial plans
* Intermediate results
* Tool outputs
* Cached reasoning

Core operations:

* `get(key)`
* `put(key, value)`
* `delete(key)`
* `list(prefix)`

---

### 2. Event Log

Purpose: Immutable, causal memory.

The event log is an append-only stream, scoped per:

* Run
* Thread
* Agent

Each event contains:

* event_id
* run_id
* timestamp
* type
* payload
* hash(prev_event)

This forms a cryptographically chainable history of what happened.

Typical use:

* Tool calls
* Observations
* Decisions
* Errors
* State transitions

Core operations:

* `append_event(run_id, event)`
* `read_events(run_id, range)`

---

### 3. State Machine Records

Purpose: Coordination between concurrent agents.

This is not just storage. It is a coordination primitive.

Each record:

* Has a name
* Has a value
* Has a version

Updates require conditional logic.

Core operations:

* `read_state(name)`
* `compare_and_swap(name, expected_version, new_value)`

This enables:

* Leader election
* Locking
* Leases
* Consensus-lite
* Workflow orchestration

---

### 4. Vector + Metadata Store

Purpose: Semantic retrieval.

Each vector:

* id
* embedding
* metadata

Search:

* brute-force initially
* metadata filters
* top-k

This is intentionally minimal.

The design assumes the index implementation will evolve, but the abstraction remains stable.

---

### 5. Trace Store

Purpose: Structured reasoning memory.

This is not logging. It is *semantic trace capture*.

Entities:

* tool_call
* model_input
* model_output
* citation
* decision
* confidence
* assumption

All traces are linked to:

* run_id
* event_id

This enables:

* Explainability
* Debugging
* Evaluation
* Replay
* Auditing

---

## Architecture Sketch

### Storage Model

* Everything lives in RAM
* Primary storage is in-memory
* Durability is achieved via an append-only WAL
* WAL can live on disk or object storage
* Snapshots are taken periodically in the background

Snapshot strategies:

* Copy-on-write pages
* Forked memory snapshots

Startup flow:

1. Load latest snapshot
2. Replay WAL
3. Resume service

---

### Concurrency Model

The system is optimized for agent access patterns:

* Many reads
* Fewer writes
* High concurrency
* Frequent conditional updates

We use **Optimistic Concurrency Control (OCC)**.

Key features:

* Snapshot isolation
* Versioned reads
* Conflict detection at commit

Why OCC:

* Agents rarely contend on the same keys
* Agents often speculate
* Retrying is cheap
* Blocking is expensive

---

### Compare-and-Swap as a First-Class Primitive

Conditional writes are not an edge case. They are the core coordination primitive.

```
cas(key, expected_version, new_value)
```

This enables:

* Atomic state transitions
* Coordination
* Locks
* Leader election
* Workflow gating

---

## Agent-Native API Surface

This system is not designed for humans writing SQL.

It is designed for agents.

### Access Modes

1. RPC / REST for tools and services
2. MCP server for coding agents

---

### Core API Patterns

#### Run Lifecycle

```
begin_run(run_id, metadata)
end_run(run_id)
```

All writes must attach to a run.

---

#### Event Capture

```
append_event(run_id, type, payload)
```

---

#### Transactions

```
txn(read_set, write_set, preconditions)
```

---

#### Coordination

```
cas(key, expected_version, new_value)
```

---

#### Retrieval

```
search_vector(collection, embedding, filter, k)
```

---

#### Replay and Diff

```
replay(run_id)
diff(run_a, run_b)
```

---

## Deterministic Replay (C)

This is a core pillar of the system.

Every run can be:

* Replayed
* Inspected
* Diffed
* Forked

Replay means:

* Same inputs
* Same tool outputs
* Same policies

Must produce:

* The same state transitions
* The same events
* The same traces

This turns agent behavior into a debuggable artifact.

---

## Safety and Governance (Pragmatic)

Agents will write nonsense. The system must assume that.

### Namespaces

Memory is partitioned by:

* Tenant
* App
* Agent
* Run

---

### TTL and Eviction

Memory is finite.

* TTL per key
* TTL per namespace
* LRU optional

---

### Write Barriers

Not all tools can write all keys.

Policies:

* Tool identity → allowed prefixes
* Agent identity → allowed scopes

---

### Audit Log

Optional at first, but architected in:

* Append-only
* Hash chained
* Tamper evident

---

## Indexing Strategy

### KV

* Hash index

### Ranges

* B-tree-like structure or skiplist (optional)

### Vectors

* Flat scan initially
* HNSW later

---

## Query Model

No SQL initially.

We use a small, composable DSL:

* filters
* projections
* sort
* limit
* range

SQL may be added later for ecosystem compatibility.

---

## A + C Positioning

This system is both:

### A) Agent Memory Database

It stores:

* Working memory
* Long-term memory
* Semantic memory
* Coordination state
* Reasoning traces

### C) Deterministic Replay Engine

It enables:

* Debugging
* Evaluation
* Auditing
* Safety analysis
* Regression testing

This combination is the wedge.

---

## Why Rust

The core engine is written in Rust because:

* Memory safety without GC
* Predictable latency
* Explicit concurrency semantics
* Determinism
* Strong correctness guarantees
* Excellent FFI

Rust allows us to treat this as a *systems project*, not a scripting project.

---

## Mental Model

This is not a database.

It is a **time machine for agent cognition**.

It lets you:

* See what an agent knew
* When it knew it
* Why it acted
* What it assumed
* What it ignored

And then replay, fork, and inspect those timelines.

---
