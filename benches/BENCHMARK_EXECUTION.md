# Benchmark Execution Prompt

Use this prompt to systematically execute the benchmark suite and document results.

---

## Execution Prompt

```
Execute the in-mem benchmark suite in the following order. Document all results.
Do NOT optimize during this run - just measure and record.

## Phase 1: Verify Correctness First

Before benchmarking, ensure the system is correct:

```bash
# Run invariant tests
cargo test --test m1_m2_comprehensive invariant -- --nocapture

# If any invariant test fails, STOP. Do not benchmark a broken system.
```

If tests fail, open an issue with label `bug`, `priority:critical` before proceeding.

## Phase 2: M1 Storage Benchmarks

Run M1 benchmarks (single-threaded, no transactions):

```bash
cargo bench --bench m1_storage -- --noplot
```

Record results for:
- [ ] `m1_kv_get/existing_key`
- [ ] `m1_kv_get/nonexistent_key`
- [ ] `m1_kv_put/unique_keys`
- [ ] `m1_kv_put/overwrite_same_key`
- [ ] `m1_kv_put/delete`
- [ ] `m1_value_size/put_bytes/*`
- [ ] `m1_wal_replay/replay_ops/*`
- [ ] `m1_memory_overhead/get_at_scale/*`

### M1 Expected Ranges

| Benchmark | Stretch | Acceptable | Concern |
|-----------|---------|------------|---------|
| kv_get | >50K ops/s | >10K ops/s | <5K ops/s |
| kv_put | >10K ops/s | >5K ops/s | <2K ops/s |
| wal_replay/50K | <500ms | <2s | >5s |

## Phase 3: M2 Transaction Benchmarks

Run M2 benchmarks (transactions, OCC, snapshots):

```bash
cargo bench --bench m2_transactions -- --noplot
```

Record results for:
- [ ] `m2_transaction_commit/single_key_put`
- [ ] `m2_transaction_commit/multi_key_put/*`
- [ ] `m2_transaction_commit/read_modify_write`
- [ ] `m2_cas/sequential_success`
- [ ] `m2_cas/failure_wrong_version`
- [ ] `m2_snapshot_read/single_read_in_txn`
- [ ] `m2_conflict_detection/no_contention_threads/*`
- [ ] `m2_conflict_detection/high_contention_threads/*`
- [ ] `m2_conflict_detection/cas_one_winner`
- [ ] `m2_version_growth/txn_after_versions/*`

### M2 Expected Ranges

| Benchmark | Stretch | Acceptable | Concern |
|-----------|---------|------------|---------|
| txn_commit (single) | >5K txns/s | >2K txns/s | <1K txns/s |
| cas_success | >5K ops/s | >2K ops/s | <1K ops/s |
| no_contention (4 threads) | >80% scaling | >60% scaling | <40% scaling |
| high_contention (4 threads) | >2K txns/s | >1K txns/s | <500 txns/s |

## Phase 4: Save Baseline

If results are acceptable, save as baseline:

```bash
cargo bench --bench m1_storage -- --save-baseline current
cargo bench --bench m2_transactions -- --save-baseline current
```

## Phase 5: Document Results

Create a benchmark report with this format:

```markdown
# Benchmark Results - [DATE]

## Environment
- OS: [uname -a]
- CPU: [model, cores]
- Memory: [total RAM]
- Rust version: [rustc --version]

## M1 Storage Results

| Benchmark | Result | vs Acceptable | Status |
|-----------|--------|---------------|--------|
| kv_get/existing | X ops/s | +Y% | OK/CONCERN |
| ... | ... | ... | ... |

## M2 Transaction Results

| Benchmark | Result | vs Acceptable | Status |
|-----------|--------|---------------|--------|
| txn_commit/single | X txns/s | +Y% | OK/CONCERN |
| ... | ... | ... | ... |

## Observations

- [Any unexpected results]
- [Bottlenecks identified]
- [Comparison to expectations]

## Action Items

- [ ] [Any issues to investigate]
- [ ] [Optimizations to consider later]
```

## Phase 6: Re-verify Correctness

After benchmarking, run invariant tests again:

```bash
cargo test --test m1_m2_comprehensive invariant -- --nocapture
```

If tests pass: benchmark results are valid.
If tests fail: benchmark results are INVALID. Something broke during the run.

---

## Interpretation Guide

### Reading Criterion Output

```
m1_kv_get/existing_key
                        time:   [1.2345 µs 1.3456 µs 1.4567 µs]
                        thrpt:  [687.29 Kelem/s 743.71 Kelem/s 810.23 Kelem/s]
```

- Use the **middle number** (estimate) for reporting
- `1.3456 µs` = ~743K ops/s
- Convert: `1,000,000 / time_in_µs = ops/s`

### Status Categories

- **OK**: Meets or exceeds "acceptable" threshold
- **MARGINAL**: Within 20% of "acceptable" threshold
- **CONCERN**: Below "acceptable" threshold
- **CRITICAL**: Below 50% of "acceptable" threshold

### What NOT to Do

1. Do NOT optimize based on a single benchmark run
2. Do NOT compare to other systems yet (we're not stable)
3. Do NOT chase "stretch" goals before "acceptable" is met
4. Do NOT ignore invariant test failures

---

## Quick Commands

```bash
# Full suite (both M1 and M2)
cargo bench --bench m1_storage --bench m2_transactions -- --noplot

# Just M1
cargo bench --bench m1_storage -- --noplot

# Just M2
cargo bench --bench m2_transactions -- --noplot

# Specific benchmark
cargo bench --bench m1_storage -- "kv_get"
cargo bench --bench m2_transactions -- "conflict_detection"

# Compare to baseline
cargo bench --bench m1_storage -- --baseline current
cargo bench --bench m2_transactions -- --baseline current

# Run with more samples (slower, more accurate)
cargo bench --bench m1_storage -- --sample-size 200

# Run invariant tests
cargo test --test m1_m2_comprehensive invariant
```

---

## Issue Template (for concerns)

If any benchmark shows "CONCERN" or "CRITICAL" status:

```markdown
## Benchmark Performance Issue

**Benchmark**: [name]
**Result**: [X ops/s]
**Expected**: [>Y ops/s (acceptable)]
**Gap**: [Z% below acceptable]

### Environment
- OS:
- Rust version:

### Reproduction
```bash
cargo bench --bench [file] -- "[benchmark_name]"
```

### Notes
[Any observations about the result]
```

Labels: `performance`, `needs-investigation`
```

---

## Success Criteria

A benchmark run is successful if:

- [ ] All invariant tests pass before AND after benchmarking
- [ ] All M1 benchmarks meet "acceptable" thresholds
- [ ] All M2 benchmarks meet "acceptable" thresholds
- [ ] No benchmark shows >20% regression from baseline (if baseline exists)
- [ ] Results are documented

If any criterion is not met, document the gap and create issues for investigation.
Do NOT block on performance issues - correctness comes first.
