# in-mem: In-Memory Database for AI Agents

A fast, durable, embedded database designed specifically for AI agent workloads with deterministic replay, run-scoped operations, and unified primitives.

## Overview

**in-mem** is a Rust-based in-memory database that treats agent runs as first-class entities. It provides six core primitives (KV store, Event Log, State Machine, Trace Store, Vector Store, Run Index) unified under a single storage layer with optimistic concurrency control and write-ahead logging for durability.

### Key Features

- **Run-Scoped Operations**: Every operation tagged with a `RunId` for deterministic replay and debugging
- **Unified Storage**: Single BTreeMap backend with type-tagged keys (enables efficient cross-primitive queries)
- **Optimistic Concurrency Control**: Non-blocking OCC with snapshot isolation (M2)
- **Durable by Default**: Write-ahead log with configurable fsync modes (strict/batched/async)
- **Deterministic Replay**: Reconstruct exact agent state from any run via Run Index
- **Embedded Library**: Zero-copy in-process API (network layer in M7)

### What Makes This Different?

Traditional databases optimize for CRUD operations on tables. **in-mem** optimizes for:

1. **Agent workflows**: Runs with parent-child relationships, forks, and lineage tracking
2. **Debugging**: Replay any run deterministically to understand agent behavior
3. **Multi-primitive coordination**: KV + Events + Traces in a single transaction
4. **Speed over perfect durability**: Batched fsync (100ms window) by default

## Architecture

### Layered Design

```
┌─────────────────────────────────────────────┐
│         API Layer (embedded/rpc/mcp)        │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│  Primitives (KV, Events, StateMachine,      │
│              Trace, RunIndex, Vector)       │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│  Engine (Database, Run Lifecycle, Coord)    │
└──┬──────────────────────────────────────┬───┘
   │                                      │
┌──▼──────────────┐            ┌──────────▼────┐
│  Concurrency    │            │  Durability   │
│  (OCC/Txn)      │            │  (WAL/Snap)   │
└─────────┬───────┘            └─────────┬─────┘
          │                              │
┌─────────▼──────────────────────────────▼─────┐
│       Storage (UnifiedStore + Indices)       │
└──────────────────────────────────────────────┘
          │
┌─────────▼──────────────────────────────────┐
│  Core Types (RunId, Key, Value, TypeTag)   │
└────────────────────────────────────────────┘
```

**See [M1_ARCHITECTURE.md](M1_ARCHITECTURE.md) for complete specification and [docs/diagrams/m1-architecture.md](docs/diagrams/m1-architecture.md) for visual architecture diagrams.**

### The Six Primitives

1. **KV Store**: Working memory, scratchpads, tool outputs
2. **Event Log**: Immutable append-only events with chaining (M3)
3. **State Machine**: CAS-based coordination records (M3)
4. **Trace Store**: Structured reasoning traces (tool calls, decisions, confidence) (M3)
5. **Run Index**: First-class run metadata with parent-child relationships and fork tracking
6. **Vector Store**: Semantic search with HNSW index (M6)

All primitives share the same unified storage layer, enabling efficient cross-primitive queries and atomic multi-primitive transactions.

## Project Status

**Current Phase**: Milestone 1 (Foundation) - In Planning

### Milestone Progress

- ✅ Architecture specification complete ([M1_ARCHITECTURE.md](M1_ARCHITECTURE.md))
- ✅ Architecture diagrams complete ([docs/diagrams/](docs/diagrams/))
- ✅ 5 Epics defined with 27 user stories ([GitHub Issues](https://github.com/anibjoshi/in-mem/issues))
- ⏳ Implementation in progress

### Roadmap to MVP

| Milestone | Goal | Status | Duration |
|-----------|------|--------|----------|
| **M1: Foundation** | Basic storage + WAL + recovery | 🔄 In Progress | Week 1-2 |
| **M2: Transactions** | OCC with snapshot isolation | 📋 Planned | Week 3 |
| **M3: Primitives** | All 5 primitives (KV, Events, SM, Trace, RunIndex) | 📋 Planned | Week 4 |
| **M4: Durability** | Snapshots + production recovery | 📋 Planned | Week 5 |
| **M5: Replay & Polish** | Deterministic replay + benchmarks | 📋 Planned | Week 6 |

**Target: 6 weeks to MVP**

See [MILESTONES.md](MILESTONES.md) for detailed milestone breakdown.

## Quick Start

**Note**: Project is in active development. This section will be updated as implementation progresses.

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo

### Installation (Planned)

```bash
# Clone the repository
git clone https://github.com/anibjoshi/in-mem.git
cd in-mem

# Build the workspace
cargo build --release

# Run tests
cargo test --all

# Run benchmarks
cargo bench
```

### Example Usage (Planned)

```rust
use in_mem::{Database, RunId};

// Open database with recovery
let db = Database::open("./data")?;

// Begin a new run
let run_id = RunId::new();
db.begin_run(run_id, metadata)?;

// Use KV primitive
db.kv().put(run_id, "key", "value")?;
let value = db.kv().get(run_id, "key")?;

// End run
db.end_run(run_id)?;

// Replay the run later
let replayed_state = db.replay_run(run_id)?;
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

## Architecture Highlights

### Design Principles

1. **Accept MVP Limitations, Design for Evolution**
   - RwLock bottleneck accepted for M1, Storage trait enables future optimization
   - Snapshot cloning expensive now, metadata enables incremental snapshots later
   - Global version counter will contend, can shard per namespace later

2. **Trait Abstractions Prevent Ossification**
   - `Storage` trait: Enables replacing BTreeMap with sharded/lock-free implementations
   - `SnapshotView` trait: Allows lazy snapshots without API changes

3. **Run-Scoped Everything**
   - All operations tagged with `RunId`
   - WAL entries include run_id for bounded replay
   - Run Index enables O(run size) replay, not O(WAL size)

4. **Conservative Recovery**
   - Discard incomplete transactions (no CommitTxn = rollback)
   - Fail-safe: corrupt entry → stop recovery, don't skip
   - CRC32 on every WAL entry

5. **Stateless Primitives**
   - Primitives are facades over Database engine
   - No cross-primitive dependencies
   - Only engine knows about run lifecycle

See [M1_ARCHITECTURE.md](M1_ARCHITECTURE.md) for complete architecture specification.

### Known Limitations (M1)

| Issue | Impact | Mitigation |
|-------|--------|------------|
| RwLock on BTreeMap | Writers block readers | Storage trait allows future replacement |
| Global version counter | AtomicU64 contention | Acceptable for MVP, can shard later |
| Snapshot serialization | Write amplification | Snapshot metadata enables incremental snapshots |
| Batched fsync (100ms) | May lose recent commits on crash | Configurable; strict mode available |

## Performance Targets

**MVP Goals (M5)**:
- Throughput: >10,000 ops/sec (single-threaded)
- Latency: <1ms p99 for KV get/put
- Recovery: <1 second for 100MB WAL
- Snapshot: <5 seconds for 1GB dataset

**Post-MVP Optimizations**:
- Sharded storage for parallel writes
- Lazy snapshots with copy-on-write
- Lock-free indices
- SIMD-optimized CRC32

## Documentation

### Architecture & Design
- **[M1_ARCHITECTURE.md](docs/architecture/M1_ARCHITECTURE.md)**: Complete M1 architecture specification
- **[Architecture Diagrams](docs/diagrams/m1-architecture.md)**: Visual architecture diagrams
- **[Original Spec](docs/architecture/spec.md)**: Original project specification

### Development Process
- **[Getting Started](docs/development/GETTING_STARTED.md)**: Quick start guide for new developers
- **[TDD Methodology](docs/development/TDD_METHODOLOGY.md)**: Test-Driven Development approach and testing strategy
- **[Development Workflow](docs/development/DEVELOPMENT_WORKFLOW.md)**: Git workflow for parallel development
- **[Claude Coordination](docs/development/CLAUDE_COORDINATION.md)**: Multi-Claude coordination guide

### Project Management
- **[Milestones](docs/milestones/MILESTONES.md)**: Project roadmap and milestone definitions
- **[Project Status](docs/milestones/PROJECT_STATUS.md)**: Current status and progress tracking
- **[Epic & User Stories](github-issues/)**: GitHub issue creation scripts

## License

[MIT License](LICENSE) (to be added)

## Contact

- **GitHub**: [anibjoshi/in-mem](https://github.com/anibjoshi/in-mem)
- **Issues**: [GitHub Issues](https://github.com/anibjoshi/in-mem/issues)

---

**Status**: Active development - M1 Foundation in progress (27 user stories defined)
