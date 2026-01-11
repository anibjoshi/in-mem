# Project Milestones: In-Memory Agent Database

## MVP Target: Single-Node, Embedded Library with Core Primitives + Replay

---

## Milestone 1: Foundation (Week 1-2)
**Goal**: Basic storage and WAL without transactions

**Deliverable**: Can store/retrieve KV pairs and append to WAL, recover from WAL on restart

**Success Criteria**:
- [ ] Cargo workspace builds
- [ ] Core types defined (RunId, Key, Value, TypeTag)
- [ ] UnifiedStore stores and retrieves values
- [ ] WAL appends entries and can be read back
- [ ] Basic recovery: restart process, replay WAL, restore state
- [ ] Unit tests pass

**Risk**: Foundation bugs will cascade. Must get this right.

---

## Milestone 2: Transactions (Week 3)
**Goal**: OCC with snapshot isolation and conflict detection

**Deliverable**: Concurrent transactions with proper isolation and rollback

**Success Criteria**:
- [ ] TransactionContext with read/write sets
- [ ] Snapshot isolation (ClonedSnapshotView)
- [ ] Conflict detection at commit
- [ ] CAS operations work
- [ ] Multi-threaded tests show proper isolation
- [ ] Conflict resolution (retry/abort) works

**Risk**: Concurrency bugs are subtle. Need thorough testing.

---

## Milestone 3: Primitives (Week 4)
**Goal**: All 5 MVP primitives working (KV, Event Log, State Machine, Trace, Run Index)

**Deliverable**: Agent can use all primitive APIs

**Success Criteria**:
- [ ] KV store: get, put, delete, list
- [ ] Event log: append, read, simple chaining (non-crypto hash)
- [ ] State machine: read_state, CAS
- [ ] Trace store: record tool calls, decisions, queries
- [ ] Run Index: create_run, get_run, update_status, query_runs
- [ ] All primitives are stateless facades over engine
- [ ] Integration tests cover primitive interactions

**Risk**: Layer boundaries. Primitives must not leak into each other.

---

## Milestone 4: Durability (Week 5)
**Goal**: Production-ready persistence with snapshots and recovery

**Deliverable**: Database survives crashes and restarts correctly

**Success Criteria**:
- [ ] Periodic snapshots (time-based and size-based)
- [ ] Snapshot metadata includes version and WAL offset
- [ ] WAL truncation after snapshot
- [ ] Full recovery: load snapshot + replay WAL
- [ ] Crash simulation tests pass
- [ ] Configurable durability modes (strict/batched/async fsync)

**Risk**: Data loss bugs. Must test recovery thoroughly.

---

## Milestone 5: Replay & Polish (Week 6)
**Goal**: Deterministic replay and production readiness

**Deliverable**: Production-ready MVP with replay

**Success Criteria**:
- [ ] replay_run(run_id) reconstructs database state
- [ ] Run Index enables O(run size) replay (not O(WAL size))
- [ ] diff_runs(run_a, run_b) compares two runs
- [ ] Example agent application works end-to-end
- [ ] Benchmarks show >10K ops/sec single-threaded
- [ ] Integration test coverage >90%
- [ ] Documentation: README, API docs, examples
- [ ] Run lifecycle (begin_run, end_run) fully working

**Risk**: Replay correctness. Must validate determinism.

---

## Post-MVP Milestones (Future)

### Milestone 6: Vector Store (Milestone 2)
- Implement vector primitive with HNSW index
- Semantic search with metadata filters
- Integration with KV/Event/Trace primitives

### Milestone 7: Network Layer (Milestone 2)
- RPC server (gRPC or similar)
- Client libraries (Rust, Python)
- Multi-client support

### Milestone 8: MCP Integration (Milestone 2)
- MCP server implementation
- Tool definitions for agent access
- IDE integration demos

### Milestone 9: Advanced Features (Milestone 3)
- Query DSL for complex filters
- Run forking and lineage tracking
- Incremental snapshots
- Sharded storage backend

---

## MVP Definition

**MVP = Milestones 1-5 Complete**

At MVP completion, the system should:
1. Store agent state in 5 primitives (KV, Events, StateMachine, Trace, RunIndex)
2. Support concurrent transactions with OCC
3. Persist data with WAL and snapshots
4. Survive crashes and recover correctly
5. Replay runs deterministically
6. Run as embedded library (single-node)
7. Achieve >10K ops/sec throughput
8. Have >90% test coverage

**Not in MVP**:
- Vector store (Milestone 6)
- Network layer (Milestone 7)
- MCP server (Milestone 8)
- Query DSL (Milestone 9)
- Distributed mode (far future)

---

## Timeline

- **Week 1-2**: Foundation (M1)
- **Week 3**: Transactions (M2)
- **Week 4**: Primitives (M3)
- **Week 5**: Durability (M4)
- **Week 6**: Replay & Polish (M5)

**Total: 6 weeks to MVP**

---

## Critical Path

```
M1 (Foundation)
  ↓
M2 (Transactions) ← Blocks M3
  ↓
M3 (Primitives) ← Blocks M5
  ↓
M4 (Durability) ← Can parallelize with M3
  ↓
M5 (Replay & Polish)
```

**Parallelization opportunity**: M4 (Durability) can start while M3 (Primitives) is being implemented, as they touch different layers.

---

## Risk Mitigation

### High-Risk Areas
1. **Concurrency (M2)**: OCC bugs are subtle
   - Mitigation: Extensive multi-threaded tests, use tools like loom
2. **Recovery (M4)**: Data loss is unacceptable
   - Mitigation: Crash simulation tests, fuzzing WAL corruption
3. **Layer boundaries (M3)**: Primitives leaking into each other
   - Mitigation: Mock tests, strict dependency rules

### Medium-Risk Areas
1. **Performance (M5)**: May not hit 10K ops/sec
   - Mitigation: Early benchmarking, profiling
2. **Replay correctness (M5)**: Determinism is hard
   - Mitigation: Property-based tests, replay verification

### Low-Risk Areas
1. **Foundation (M1)**: Well-understood patterns
2. **API design (M3)**: Can iterate post-MVP
