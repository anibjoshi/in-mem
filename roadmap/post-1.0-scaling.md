# Post-1.0: Scaling Beyond Single-Node

**Theme**: Scaling beyond single-node embedded use cases.

These features are on the radar but not committed to any milestone. They represent potential directions depending on how the AI agent ecosystem evolves.

---

## Agent Runtime

Branch-native planning and evaluation engine.

### Parallel Planning (Speculative Execution)

Fork N speculative branches, run agents concurrently, select the best outcome, merge the winner back.

```
           base branch
          ╱     │      ╲
    plan-a   plan-b   plan-c     ← fork_many
       │        │        │
    [agent]  [agent]  [agent]    ← concurrent execution
       │        │        │
    score=7  score=9  score=4    ← evaluate
                │
             merge ──→ base      ← merge winner
```

### Evaluation Harness

Orchestrate N runs across branches with different configs. Compare outcomes. Preserve artifacts. Built-in metrics (success/failure, latency, tool count) plus custom evaluators via SDK hooks.

### Branch Cloud Sync

Push and pull branches to remote registries. Local-first speed with cloud collaboration. Delta sync via WAL segments.

---

## Server Mode

Optional network layer for multi-process access. One process hosts the database, others connect over TCP.

- Standalone binary: `stratadb-server --data-dir /path --port 9876`
- MessagePack wire protocol over length-prefixed TCP frames
- Rust client crate with the same `Strata` API (one-line switch from embedded)
- Per-connection authentication and access mode
- Only if there's demonstrated demand

## Replication

- Stream WAL entries from primary to replicas for read scaling and automatic failover

## Horizontal Sharding

- Branches as the natural sharding unit across nodes
- No cross-branch transactions (already the case)

## Cloud-Managed Service

- Multi-tenant with branch-level isolation, managed backups, usage-based pricing

## WASM Build

- Compile to `wasm32-unknown-unknown` for browser-embedded use (Ephemeral mode only)
- Powers live demos on stratadb.ai

## Websites

- **stratadb.org**: Static docs site from `docs/` and `roadmap/` markdown
- **stratadb.ai**: Live interactive demo running Strata via WebAssembly in the browser

## Public Graph Operations

- Promote the internal knowledge graph (v0.8) to a public primitive if demand warrants it

---

## How features move from here to a milestone

A feature moves from post-1.0 to a committed milestone when:

1. There is demonstrated user demand
2. The prerequisite infrastructure exists
3. The design is well-understood enough to scope
4. It aligns with the project's current priorities
