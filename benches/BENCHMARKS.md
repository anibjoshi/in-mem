# in-mem Benchmark Suite

This document describes the benchmark suite for the in-mem database.

**Philosophy:** MVP success is semantic correctness first, performance second.
Benchmarks exist to detect regressions, not to chase arbitrary numbers.

---

## Benchmark Structure

Benchmarks are organized by milestone to match feature development:

```
benches/
  m1_storage.rs         # M1: Storage + WAL primitives
  m2_transactions.rs    # M2: OCC + Snapshot Isolation
  comprehensive_benchmarks.rs  # Future: M3+ scenarios
```

### Why milestone-scoped benchmarks?

1. **Focus:** Only benchmark what's implemented
2. **Avoid distraction:** Don't optimize for features that don't exist yet
3. **Clear ownership:** Each benchmark file maps to a feature set
4. **Regression detection:** Changes to M1 run M1 benchmarks

---

## Target Performance

### Important Context

These targets assume:
- Single-process, in-memory
- RwLock-based concurrency
- BTreeMap-backed storage
- WAL-logged mutations
- Versioned values with snapshot isolation

**Stretch goals are optimistic.** Initial implementations may be 2-5x slower.
That's fine. Correctness first.

### M1: Storage + WAL

| Operation     | Stretch Goal   | Acceptable     | Notes                    |
|---------------|----------------|----------------|--------------------------|
| KV get        | 50-200K ops/s  | 10-40K ops/s   | RwLock + BTreeMap lookup |
| KV put        | 10-50K ops/s   | 5-20K ops/s    | + WAL append             |
| KV delete     | 10-50K ops/s   | 5-20K ops/s    | + tombstone              |
| WAL replay    | <300ms/1M ops  | <1s/1M ops     | Cold start recovery      |

### M2: Transactions + OCC

| Operation                | Stretch Goal   | Acceptable     | Notes                    |
|--------------------------|----------------|----------------|--------------------------|
| Txn commit (no conflict) | 5-10K txns/s   | 2-5K txns/s    | Single-threaded          |
| Txn commit (conflict)    | 2-5K txns/s    | 1-2K txns/s    | With retry overhead      |
| CAS                      | 5-20K ops/s    | 2-5K ops/s     | + version validation     |
| Snapshot read            | 50-100K ops/s  | 20-50K ops/s   | No conflict possible     |
| Multi-thread (no conflict)| 80% of 1-thread| 60% of 1-thread| Scaling efficiency       |

### Future (M3+)

| Metric              | Notes                                         |
|---------------------|-----------------------------------------------|
| Event append        | Not implemented yet                           |
| Event scan          | Not implemented yet                           |
| Agent scenarios     | Deferred until primitives are stable          |
| Durability modes    | Deferred until core is validated              |

---

## Running Benchmarks

### M1 Storage Benchmarks

```bash
# All M1 benchmarks
cargo bench --bench m1_storage

# Specific categories
cargo bench --bench m1_storage -- m1_kv_get
cargo bench --bench m1_storage -- m1_kv_put
cargo bench --bench m1_storage -- m1_wal_replay
cargo bench --bench m1_storage -- m1_memory_overhead
```

### M2 Transaction Benchmarks

```bash
# All M2 benchmarks
cargo bench --bench m2_transactions

# Specific categories
cargo bench --bench m2_transactions -- m2_transaction_commit
cargo bench --bench m2_transactions -- m2_cas
cargo bench --bench m2_transactions -- m2_snapshot_read
cargo bench --bench m2_transactions -- m2_conflict_detection
cargo bench --bench m2_transactions -- m2_version_growth
```

### Comparison Mode

```bash
# Save baseline
cargo bench --bench m1_storage -- --save-baseline main

# Compare against baseline
cargo bench --bench m1_storage -- --baseline main
```

---

## What Each Benchmark Measures

### M1 Storage

| Benchmark | What It Measures | Why It Matters |
|-----------|------------------|----------------|
| `kv_get/existing_key` | BTreeMap lookup latency | Hot path performance |
| `kv_get/nonexistent_key` | Miss path latency | Error handling overhead |
| `kv_get/position/*` | Lookup at different tree positions | BTree traversal consistency |
| `kv_put/unique_keys` | Append workload throughput | Event log pattern |
| `kv_put/overwrite_same_key` | Update workload throughput | State update pattern |
| `kv_put/delete` | Delete throughput | Cleanup performance |
| `value_size/put_bytes/*` | Impact of value size | Large value handling |
| `wal_replay/replay_ops/*` | Cold start time | Recovery performance |
| `wal_replay/replay_mixed` | Recovery with deletes | Realistic recovery |
| `memory_overhead/get_at_scale` | Lookup as key count grows | Scaling behavior |

### M2 Transactions

| Benchmark | What It Measures | Why It Matters |
|-----------|------------------|----------------|
| `transaction_commit/single_key_put` | Basic txn overhead | Minimum cost |
| `transaction_commit/multi_key_put/*` | Atomic batch overhead | Real-world pattern |
| `transaction_commit/read_modify_write` | RMW pattern throughput | Counter/state updates |
| `cas/sequential_success` | CAS happy path | Atomic updates |
| `cas/failure_wrong_version` | CAS failure handling | Conflict cost |
| `cas/create_new_key` | Insert-if-absent pattern | Initialization |
| `snapshot_read/single_read_in_txn` | Snapshot read latency | Read-heavy workloads |
| `snapshot_read/read_your_writes` | Write visibility | Consistency check |
| `conflict_detection/no_contention_threads` | Parallel scaling | Throughput ceiling |
| `conflict_detection/high_contention_threads` | Conflict overhead | Worst case |
| `conflict_detection/cas_one_winner` | CAS correctness + perf | Invariant validation |
| `version_growth/txn_after_versions` | Version history impact | Long-running systems |
| `version_growth/snapshot_read_versioned` | MVCC overhead | Snapshot isolation cost |

---

## Interpreting Results

### Criterion Output

```
m1_kv_get/existing_key
                        time:   [1.2345 µs 1.3456 µs 1.4567 µs]
                        thrpt:  [687.29 Kelem/s 743.71 Kelem/s 810.23 Kelem/s]
```

- Three numbers: [lower bound, estimate, upper bound] at 95% confidence
- `thrpt` = throughput in elements/second
- 687K ops/s = well above "acceptable" (10-40K ops/s)

### Regression Detection

```
Performance has regressed:
  time:   [1.2345 µs 1.3456 µs 1.4567 µs]
                        change: [+15.234% +18.901% +22.345%] (p = 0.001 < 0.05)
```

- `change` shows percentage difference from baseline
- `p < 0.05` means statistically significant
- Investigate regressions >10% on critical paths

### What to Do About Regressions

1. **<5%:** Noise, ignore
2. **5-15%:** Investigate, may be acceptable tradeoff
3. **>15%:** Likely real regression, prioritize fix
4. **>50%:** Something is seriously wrong

---

## What's NOT Benchmarked (Yet)

### Variance / Tail Latency

We don't yet measure:
- P95, P99 latency under load
- Jitter during concurrent access
- Worst-case pauses
- Snapshot pause time
- TTL cleanup spikes

**Why:** These require more sophisticated harnesses and longer runs.
Add when correctness is proven.

### Pathological Cases

We don't yet measure:
- Large values (1MB+)
- Very long key prefixes
- Highly skewed key access (zipfian)
- CAS storms

**Why:** These are optimization targets after core is stable.

### Comparison to Other Systems

We don't yet compare to:
- Redis (networked, different tradeoffs)
- SQLite (relational overhead)
- RocksDB/Badger (disk-optimized)

**Why:** Comparisons are only meaningful after our system is stable.
When we do compare, always contextualize:
- Redis is networked, we're in-process
- SQLite is relational, we're KV + transactions
- RocksDB is disk-first, we're memory-first

---

## CI Integration

### Recommended CI Job

```yaml
benchmark:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Run M1 benchmarks
      run: cargo bench --bench m1_storage -- --noplot

    - name: Run M2 benchmarks
      run: cargo bench --bench m2_transactions -- --noplot

    # Optional: fail on regression
    - name: Check for regressions
      run: |
        cargo bench --bench m1_storage -- --baseline main || true
        cargo bench --bench m2_transactions -- --baseline main || true
```

### Thresholds

Don't fail CI on benchmark regressions until the system is stable.
Use benchmarks for visibility, not gates.

---

## Adding New Benchmarks

### When to Add

- After implementing a new feature
- After finding a performance-related bug
- When a workload pattern becomes common

### Template

```rust
fn my_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("category_name");

    // Setup
    let temp_dir = TempDir::new().unwrap();
    let db = Database::open(temp_dir.path().join("db")).unwrap();

    // Benchmark
    group.throughput(Throughput::Elements(1));
    group.bench_function("benchmark_name", |b| {
        b.iter(|| {
            // Operation to benchmark
            black_box(result);
        });
    });

    group.finish();
}
```

### Checklist

- [ ] Does it measure something meaningful for users?
- [ ] Is the setup realistic?
- [ ] Does it belong in the current milestone?
- [ ] Is there an existing benchmark that covers this?

---

## Invariant Validation After Benchmarks

**Performance without correctness is meaningless.**

After running benchmarks, validate invariants:

```bash
# Run invariant tests
cargo test --test m1_m2_comprehensive invariant

# Quick sanity check
cargo test --test m1_m2_comprehensive -- --ignored stress
```

If benchmarks pass but invariant tests fail, the benchmarks are measuring a broken system.
