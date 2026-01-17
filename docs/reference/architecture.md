# Architecture Overview

Learn how **in-mem** works internally and why it's designed the way it is.

**Current Version**: 0.5.0 (M5 JSON + M6 Retrieval)

## Design Philosophy

1. **Run-First Design**: Every operation is scoped to a run for deterministic replay
2. **Layered Performance**: Fast paths for common operations, full transactions when needed
3. **Accept MVP Limitations, Design for Evolution**: Simple implementations now, trait abstractions for future optimization

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│              API Layer (embedded/rpc/mcp)               │
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│  Primitives (KV, EventLog, StateCell, Trace, RunIndex,  │  ← Stateless facades
│              JsonStore)                                 │
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│  Search Layer (HybridSearch, BM25, InvertedIndex, RRF)  │  ← Retrieval surfaces
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│       Engine (Database, Run Lifecycle, Coordinator)     │  ← Orchestration
└───────┬───────────────────────────────────────┬─────────┘
        │                                       │
┌───────▼───────────────┐         ┌─────────────▼─────────┐
│     Concurrency       │         │      Durability       │
│  (OCC/Transactions)   │         │  (InMemory/Buffered/  │
│                       │         │       Strict)         │
└───────────┬───────────┘         └───────────┬───────────┘
            │                                 │
┌───────────▼─────────────────────────────────▼───────────┐
│         Storage (UnifiedStore + Snapshots)              │
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│      Core Types (RunId, Key, Value, TypeTag)            │
└─────────────────────────────────────────────────────────┘
```

## Concurrency Model

### Optimistic Concurrency Control (OCC)

**in-mem** uses OCC with first-committer-wins conflict detection:

1. **BEGIN**: Acquire snapshot (current version)
2. **EXECUTE**: Read from snapshot, buffer writes
3. **VALIDATE**: Check read_set versions unchanged
4. **COMMIT**: Allocate version, write to WAL, apply to storage

### Read-Your-Writes Semantics

Within a transaction, reads see uncommitted writes:
1. Check `write_set` (uncommitted write)
2. Check `delete_set` (uncommitted delete → return None)
3. Check snapshot (committed data)

## Durability Modes (M4)

### InMemory Mode

```
write → apply to storage → return
```

- Latency: <3µs
- Throughput: 250K+ ops/sec
- Data Loss: All (on crash)

### Buffered Mode (Production Default)

```
write → log to WAL buffer → apply to storage → return
                 ↓
      background thread fsyncs periodically
```

- Latency: <30µs
- Throughput: 50K+ ops/sec
- Data Loss: Bounded (~100ms)

### Strict Mode

```
write → log to WAL → fsync → apply to storage → return
```

- Latency: ~2ms
- Throughput: ~500 ops/sec
- Data Loss: Zero

## Primitives Architecture

All six primitives are stateless facades:

```rust
pub struct Primitive {
    db: Arc<Database>
}
```

**Six Primitives**:
- **KVStore**: Key-value storage with batch operations
- **EventLog**: Append-only log with hash chaining
- **StateCell**: Named cells with CAS operations
- **TraceStore**: Hierarchical trace recording
- **RunIndex**: Run lifecycle management
- **JsonStore** (M5): JSON documents with path mutations

### Fast Path vs Transaction Path

**Fast Path** (for read-only operations):
- Direct snapshot read
- No transaction overhead
- <10µs latency

**Transaction Path** (for writes):
- Full OCC with conflict detection
- WAL persistence (based on durability mode)

## Search Architecture (M6)

### Hybrid Search

**in-mem** provides unified search across all primitives:

```
SearchRequest → HybridSearch → [BM25 + Semantic] → RRF Fusion → SearchResponse
```

### Components

**BM25Lite**: Lightweight keyword scoring
- Tokenization with lowercase normalization
- TF-IDF weighting with BM25 formula
- Title boost for structured documents

**InvertedIndex**: Optional full-text index
- Disabled by default (opt-in)
- Tracks document frequency and term positions
- Version-based cache invalidation

**RRF Fusion**: Reciprocal Rank Fusion
- Combines keyword and semantic scores
- Default k=60 for rank normalization
- Preserves relative ordering from both sources

### Budget Semantics

Search operations respect time budgets:
- `budget_ms`: Maximum search time
- Graceful degradation on timeout
- Partial results returned with budget metadata

## Performance Characteristics

| Metric | Target |
|--------|--------|
| InMemory put | <3µs |
| InMemory throughput (1 thread) | 250K ops/sec |
| Buffered put | <30µs |
| Buffered throughput | 50K ops/sec |
| Fast path read | <10µs |
| Disjoint scaling (4 threads) | ≥3.2× |
| Search (no index) | O(n) scan |
| Search (with index) | O(log n) lookup |

## See Also

- [API Reference](api-reference.md)
- [Getting Started Guide](getting-started.md)
- [Milestones](../milestones/MILESTONES.md)
