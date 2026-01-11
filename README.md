# in-mem: State Substrate for AI Agents

**A memory, coordination, and replay foundation for building reliable AI agents**

## The Problem

AI agents today are non-deterministic black boxes. When they fail, you can't replay what happened. When they coordinate (like handling tool calls or managing state machines), you build fragile locking on top of Redis. When you need to debug multi-step reasoning, you're stuck with scattered logs.

**in-mem** solves this by giving agents what operating systems give programs: durable memory, safe coordination primitives, and deterministic replay.

## What is in-mem?

**in-mem is not a traditional database.** It's a state substrate for AI agents that need:

- **Durable Memory**: KV storage and event logs that survive crashes
- **Safe Coordination**: Lock-free primitives for managing state machines and tool outputs
- **Deterministic Replay**: Reconstruct any agent execution exactly, like Git for runs

Think of runs as commits. Every agent execution is a `RunId`—a first-class entity you can replay, diff, fork, and debug. Just like you can `git checkout` any commit, you can replay any run and see exactly what the agent did.

### For Whom?

**in-mem is for people building agents**, not using them. If you're:

- Building an agent framework and need reliable state management
- Debugging why your agent made a decision 3 tool calls ago
- Coordinating between multiple agents or LLM calls
- Implementing deterministic testing for agent workflows

...then in-mem provides the substrate you'd otherwise build yourself on Redis + Postgres + custom replay logic.

### What in-mem Is NOT

- **Not a vector database**: Use Qdrant/Pinecone for embeddings (we complement them)
- **Not a general-purpose database**: Use Postgres/MySQL for application data
- **Not a cache**: Use Redis for hot ephemeral data
- **Not LangGraph/LangChain**: We're the state layer they can build on

**in-mem sits below agent frameworks**, providing the durable memory and replay guarantees they need.

## Core Capabilities

### 1. Runs as First-Class Entities

Every operation is scoped to a `RunId`. Runs have:
- **Parent-child relationships**: Fork runs, track lineage
- **Bounded replay**: Replay only what this run touched (not entire history)
- **Metadata**: Tags, timestamps, status, retention policies

```rust
let run_id = db.begin_run();
db.kv().put(run_id, "state", "thinking")?;
db.events().append(run_id, ToolCallEvent { ... })?;
db.end_run(run_id)?;

// Later: replay this exact run
let snapshot = db.replay_run(run_id)?;
```

### 2. Unified Primitives for Agent State

Six primitives sharing one storage layer:

1. **KV Store**: Working memory, tool outputs, scratchpads
2. **Event Log**: Immutable history (tool calls, decisions)
3. **State Machine**: CAS-based coordination (managing multi-step flows)
4. **Trace Store**: Structured reasoning (confidence scores, alternatives)
5. **Run Index**: First-class run metadata and relationships
6. **Vector Store**: Semantic search (coming in M2)

All transactional. All replay-able. All tagged with the run that created them.

### 3. Deterministic Replay

Reproduce any agent execution exactly:

```rust
// Original run
let run_id = db.begin_run();
agent.execute(run_id)?;
db.end_run(run_id)?;

// Replay later (deterministic)
let state = db.replay_run(run_id)?;
assert_eq!(state, original_state);

// Diff two runs
let diff = db.diff_runs(run_a, run_b)?;
```

This makes agents **debuggable** and **testable** in ways they've never been before.

## Why Not Just Use Redis + Postgres?

You *can* build this yourself. Most agent frameworks do. But you'll end up with:

- **Fragile replay**: Scanning logs and hoping you capture everything
- **Locking hell**: Redis locks for coordination, race conditions everywhere
- **No causality**: Events in Postgres have timestamps, not causal relationships
- **Manual versioning**: Tracking what changed when, rolling back partial runs

**in-mem gives you all of this out of the box**, designed for agents from the ground up

## How It Works

**in-mem** is built in layers, with runs and causality baked into every level:

```
┌──────────────────────────────────────────────┐
│     Your Agent Framework / Application       │  ← You build here
└──────────────────┬───────────────────────────┘
                   │
┌──────────────────▼───────────────────────────┐
│  Primitives: KV, Events, State Machine,      │  ← High-level APIs
│              Trace, Run Index, Vector        │
└──────────────────┬───────────────────────────┘
                   │
┌──────────────────▼───────────────────────────┐
│  Engine: Run Lifecycle, Transactions, Replay │  ← Orchestration
└───┬──────────────────────────────────────┬───┘
    │                                      │
┌───▼─────────────┐            ┌───────────▼───┐
│  Concurrency    │            │  Durability   │  ← Guarantees
│  (OCC)          │            │  (WAL)        │
└───┬─────────────┘            └───────────┬───┘
    │                                      │
┌───▼──────────────────────────────────────▼───┐
│  Unified Storage: Run-tagged BTreeMap        │  ← State
└──────────────────────────────────────────────┘
```

**Key Design Choices**:

1. **Unified Storage**: All primitives share one sorted map (not separate stores). This enables atomic multi-primitive transactions and efficient cross-primitive queries.

2. **Run-Tagged Keys**: Every key includes its `RunId`. This makes replay O(run size), not O(history size).

3. **Optimistic Concurrency**: Lock-free transactions with compare-and-swap. Agents rarely conflict; when they do, we retry.

4. **Batched Durability**: fsync every 100ms by default (not every write). Agents prefer speed; losing 100ms of work is acceptable.

See [Architecture Overview](docs/reference/architecture.md) for technical details and [M1_ARCHITECTURE.md](docs/architecture/M1_ARCHITECTURE.md) for the complete specification.

## Project Status

**Current Phase**: ✅ **M1 Foundation Complete!**

### M1 Achievements

- ✅ **297 total tests** (95.45% coverage)
- ✅ **Performance**: 20,564 txns/sec recovery (10x over target)
- ✅ **Zero compiler warnings** (clippy clean)
- ✅ **TDD integrity verified** (9-phase quality audit passed)
- ✅ All integration tests passing

**See**: [PROJECT_STATUS.md](docs/milestones/PROJECT_STATUS.md) for full details.

### Roadmap to MVP

| Milestone | Goal | Status | Duration |
|-----------|------|--------|----------|
| **M1: Foundation** | Basic storage + WAL + recovery | ✅ **Complete** | 2 days |
| **M2: Transactions** | OCC with snapshot isolation | 📋 Next | Week 3 |
| **M3: Primitives** | All 5 primitives (KV, Events, SM, Trace, RunIndex) | 📋 Planned | Week 4 |
| **M4: Durability** | Snapshots + production recovery | 📋 Planned | Week 5 |
| **M5: Replay & Polish** | Deterministic replay + benchmarks | 📋 Planned | Week 6 |

**Target: M1 complete, M2-M5 in progress**

See [MILESTONES.md](docs/milestones/MILESTONES.md) for detailed milestone breakdown.

## Quick Start

**Note**: M1 Foundation is complete but not yet published to crates.io. Coming soon.

```rust
use in_mem::Database;

// Open database (auto-recovers from crashes)
let db = Database::open("./agent-state")?;

// Every agent execution is a run
let run_id = db.begin_run();

// Use primitives to manage state
db.put(run_id, b"thinking", b"analyzing user query")?;
db.put(run_id, b"tool_result", b"{...}")?;

// Retrieve state
let state = db.get(run_id, b"thinking")?;

// End the run (makes it replay-able)
db.end_run(run_id)?;

// Later: replay this exact execution
let replayed = db.replay_run(run_id)?;
```

**📚 Full Documentation**:
- [Getting Started Guide](docs/reference/getting-started.md) - Installation, patterns, best practices
- [API Reference](docs/reference/api-reference.md) - Complete API documentation
- [Architecture Overview](docs/reference/architecture.md) - How in-mem works internally

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
in-mem = "0.1"
```

Or clone and build:

```bash
git clone https://github.com/anibjoshi/in-mem.git
cd in-mem
cargo build --release
cargo test --all

# Run benchmarks
cargo bench
```

### Example: Multi-Agent Coordination (Planned for M3)

```rust
use in_mem::{Database, primitives::*};

let db = Database::open("./agent-cluster")?;

// Agent 1: Claim a task using state machine CAS
let run_1 = db.begin_run();
let claimed = db.state_machine().cas(
    run_1,
    "task:123:status",
    "pending",  // expected
    "claimed_by_agent_1"  // new value
)?;

if claimed {
    // Execute task, log events
    db.events().append(run_1, ToolCallEvent { ... })?;
    db.kv().put(run_1, "task:123:result", result)?;
    db.state_machine().set(run_1, "task:123:status", "completed")?;
}
db.end_run(run_1)?;

// Agent 2: Sees updated state, different run
let run_2 = db.begin_run();
let status = db.state_machine().get(run_2, "task:123:status")?;
assert_eq!(status, "completed");

// Later: replay both runs to debug coordination
db.replay_run(run_1)?;
db.replay_run(run_2)?;
```

## Development

### Workspace Structure

```
in-mem/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── core/                     # Core types and traits
│   ├── storage/                  # UnifiedStore + indices
│   ├── concurrency/              # OCC transactions (M2)
│   ├── durability/               # WAL + snapshots
│   ├── primitives/               # 6 primitives
│   ├── engine/                   # Database orchestration
│   └── api/                      # Public API
├── examples/                     # Usage examples
├── tests/                        # Integration tests
├── benches/                      # Benchmarks
└── docs/                         # Documentation
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Crash simulation tests (M1)
cargo test --test crash_simulation

# Corruption simulation tests (M1)
cargo test --test corruption_simulation
```

### Contributing

This project follows a structured development process:

1. **Milestones**: High-level goals (M1-M5)
2. **Epics**: Feature areas within each milestone
3. **User Stories**: Specific deliverables with acceptance criteria

See [GitHub Issues](https://github.com/anibjoshi/in-mem/issues) for current work items.

**Development Flow**:
- All work tracked as GitHub issues
- Branch naming: `epic-N-story-M-brief-description`
- Pull requests reference issue numbers
- All PRs require tests and documentation

## Design Philosophy

**in-mem** is built around three principles:

### 1. Runs Are First-Class

Not just identifiers—runs have:
- Metadata (tags, status, timestamps)
- Relationships (parent-child, forks)
- Boundaries (first/last version, WAL offsets)

This makes replay O(run size), not O(history size). You can diff two runs, fork from a checkpoint, or query "show me all failed runs."

### 2. Accept MVP Limits, Design for Evolution

- **M1**: Single BTreeMap + RwLock (simple, correct, will bottleneck under load)
- **Future**: Storage trait enables swap to sharded/lock-free without breaking API
- **M1**: Clone entire map for snapshots (expensive, but works)
- **Future**: Snapshot metadata enables incremental snapshots later

Ship fast, but design for the future you'll need.

### 3. Speed Over Perfect Durability

Agents prefer 100 μs writes over perfect durability. Default: fsync every 100ms (batched mode). You can lose 100ms of work on crash—that's acceptable. Financial ledgers use strict mode; agents use batched.

See [M1_ARCHITECTURE.md](docs/architecture/M1_ARCHITECTURE.md) for the complete technical specification.

### Known Limitations (M1)

| Issue | Impact | Mitigation |
|-------|--------|------------|
| RwLock on BTreeMap | Writers block readers | Storage trait allows future replacement |
| Global version counter | AtomicU64 contention | Acceptable for MVP, can shard later |
| Snapshot serialization | Write amplification | Snapshot metadata enables incremental snapshots |
| Batched fsync (100ms) | May lose recent commits on crash | Configurable; strict mode available |

## Performance

**Current (M1)**:
- **20,564 txns/sec** recovery throughput (10x over target)
- **95.45% test coverage** across 297 tests
- **Zero compiler warnings** (clippy clean)

**Target for MVP (M5)**:
- **10K+ ops/sec** single-threaded (KV put/get)
- **<1ms p99** latency for operations
- **<1 second** recovery for 100MB WAL
- **O(run size)** replay (not O(history))

**Known Bottlenecks** (accepted for M1, will optimize later):
- RwLock on BTreeMap (writers block readers)
- Global version counter (AtomicU64 contention)
- Snapshot cloning (entire map copied)

These are acceptable for embedded use. Future milestones will add sharding, lock-free structures, and lazy snapshots.

## Documentation

### 📖 User Documentation

**Start here** for using **in-mem** in your projects:

- **[Reference Documentation](docs/reference/)** - Complete user guides
  - [Getting Started](docs/reference/getting-started.md) - Quick start, installation, common patterns
  - [API Reference](docs/reference/api-reference.md) - Complete API documentation
  - [Architecture Overview](docs/reference/architecture.md) - How in-mem works

### 🔧 Developer Documentation

**For contributors** building in-mem:

- [M1 Architecture Spec](docs/architecture/M1_ARCHITECTURE.md) - Detailed technical specification
- [Development Workflow](docs/development/DEVELOPMENT_WORKFLOW.md) - Git workflow and contribution guide
- [TDD Methodology](docs/development/TDD_METHODOLOGY.md) - Testing strategy and best practices
- [Developer Onboarding](docs/development/GETTING_STARTED.md) - Setup for new contributors

### 📊 Project Documentation

**Project status and planning**:

- [Project Status](docs/milestones/PROJECT_STATUS.md) - Current development status
- [Milestones](docs/milestones/MILESTONES.md) - Roadmap M1-M5 with timeline
- [Architecture Diagrams](docs/diagrams/m1-architecture.md) - Visual system diagrams

### 📦 Historical Documentation

**M1 development artifacts** preserved in [docs-archive branch](https://github.com/anibjoshi/in-mem/tree/docs-archive):

- M1 Completion Report (541 lines) - Epic results and benchmarks
- 9-Phase Quality Audit Report (980 lines) - Comprehensive validation
- Epic Reviews and Summaries - Development retrospectives
- Claude Coordination Prompts - Multi-agent implementation guides

## Roadmap

**Milestone 1 (M1): Foundation** ✅ Complete
- Core storage with WAL and recovery
- Run lifecycle and metadata
- Basic KV primitive
- 297 tests, 95.45% coverage

**Milestone 2 (M2): Transactions** 📋 Next (Week 3)
- Optimistic Concurrency Control (OCC)
- Snapshot isolation
- Multi-key atomic transactions

**Milestone 3 (M3): Primitives** 📋 Planned (Week 4)
- Event Log with chaining
- State Machine with CAS
- Trace Store for reasoning
- Run Index with fork tracking

**Milestone 4 (M4): Production Durability** 📋 Planned (Week 5)
- Periodic snapshots
- WAL truncation
- Incremental snapshot support

**Milestone 5 (M5): Replay & Polish** 📋 Planned (Week 6)
- Deterministic replay implementation
- Run diffing
- Performance benchmarks
- Documentation polish

**Beyond MVP**:
- M6: Vector Store (HNSW index)
- M7: Network layer (RPC + MCP)
- M8: Distributed mode
- M9: Encryption at rest

See [MILESTONES.md](docs/milestones/MILESTONES.md) for details.

## FAQ

**Q: Is this a replacement for Redis/Postgres?**
A: No. in-mem complements traditional databases. Use Postgres for application data, Redis for caching, Qdrant for vectors. Use in-mem for agent state that needs replay and coordination.

**Q: Why not just use SQLite?**
A: SQLite is great for relational data but doesn't have run-scoped operations, deterministic replay, or causality tracking built in. You'd build in-mem's features yourself on top of SQLite.

**Q: Is this production-ready?**
A: M1 is production-ready for embedded use (297 tests, 95% coverage, crash recovery verified). Network layer and distributed mode come later.

**Q: What about horizontal scaling?**
A: M1 is embedded (in-process). M8+ will add distributed mode. For now, use multiple in-mem instances with agent-level sharding.

**Q: Can I use this with LangChain/LangGraph?**
A: Yes! in-mem sits below agent frameworks. They can use in-mem for state management instead of building custom persistence.

## License

[MIT License](LICENSE) (to be added)

## Contact

- **GitHub**: [anibjoshi/in-mem](https://github.com/anibjoshi/in-mem)
- **Issues**: [GitHub Issues](https://github.com/anibjoshi/in-mem/issues)

---

**Status**: ✅ M1 Foundation Complete | 📋 M2 Planning Phase
