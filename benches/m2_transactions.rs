//! M2 Transaction Benchmarks
//!
//! These benchmarks focus on M2 (OCC + Snapshot Isolation) primitives.
//! Includes: transactions, conflict detection, CAS, version growth.
//!
//! ## What M2 is:
//! - Snapshot isolation
//! - Optimistic concurrency control
//! - First-committer-wins conflict detection
//! - CAS semantics
//!
//! ## Target Performance (MVP)
//!
//! | Operation                | Stretch Goal   | Acceptable     | Notes                    |
//! |--------------------------|----------------|----------------|--------------------------|
//! | Txn commit (no conflict) | 5-10K txns/s   | 2-5K txns/s    | Single-threaded          |
//! | Txn commit (conflict)    | 2-5K txns/s    | 1-2K txns/s    | With retry overhead      |
//! | CAS                      | 5-20K ops/s    | 2-5K ops/s     | + version validation     |
//! | Snapshot read            | 50-100K ops/s  | 20-50K ops/s   | No conflict possible     |
//!
//! **These are stretch goals. MVP success is semantic correctness first,
//! performance second.**
//!
//! ## Running
//!
//! ```bash
//! cargo bench --bench m2_transactions
//! ```

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use in_mem_core::types::{Key, Namespace, RunId};
use in_mem_core::value::Value;
use in_mem_engine::Database;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

// =============================================================================
// Test Utilities
// =============================================================================

fn create_namespace(run_id: RunId) -> Namespace {
    Namespace::new(
        "tenant".to_string(),
        "app".to_string(),
        "agent".to_string(),
        run_id,
    )
}

fn make_key(ns: &Namespace, name: &str) -> Key {
    Key::new_kv(ns.clone(), name)
}

// =============================================================================
// Transaction Commit (Single-Threaded, No Conflict)
// =============================================================================

fn transaction_commit_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_transaction_commit");
    group.throughput(Throughput::Elements(1));

    // Single-key transaction
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_function("single_key_put", |b| {
            let counter = AtomicU64::new(0);
            b.iter(|| {
                let i = counter.fetch_add(1, Ordering::Relaxed);
                let result = db.transaction(run_id, |txn| {
                    let key = make_key(&ns, &format!("txn_{}", i));
                    txn.put(key, Value::I64(i as i64))?;
                    Ok(())
                });
                black_box(result.unwrap());
            });
        });
    }

    // Multi-key transaction (atomic batch)
    for num_keys in [3, 5, 10] {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_with_input(
            BenchmarkId::new("multi_key_put", num_keys),
            &num_keys,
            |b, &num_keys| {
                let counter = AtomicU64::new(0);
                b.iter(|| {
                    let i = counter.fetch_add(1, Ordering::Relaxed);
                    let result = db.transaction(run_id, |txn| {
                        for j in 0..num_keys {
                            let key = make_key(&ns, &format!("batch_{}_{}", i, j));
                            txn.put(key, Value::I64((i * num_keys as u64 + j as u64) as i64))?;
                        }
                        Ok(())
                    });
                    black_box(result.unwrap());
                });
            },
        );
    }

    // Read-modify-write transaction
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);
        let key = make_key(&ns, "rmw_key");
        db.put(run_id, key.clone(), Value::I64(0)).unwrap();

        group.bench_function("read_modify_write", |b| {
            b.iter(|| {
                let result = db.transaction(run_id, |txn| {
                    let val = txn.get(&key)?;
                    let n = match val {
                        Some(Value::I64(n)) => n,
                        _ => 0,
                    };
                    txn.put(key.clone(), Value::I64(n + 1))?;
                    Ok(())
                });
                black_box(result.unwrap());
            });
        });
    }

    group.finish();
}

// =============================================================================
// CAS Performance
// =============================================================================

fn cas_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_cas");
    group.throughput(Throughput::Elements(1));

    // CAS success (sequential, no conflict)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);
        let key = make_key(&ns, "cas_key");
        db.put(run_id, key.clone(), Value::I64(0)).unwrap();

        group.bench_function("sequential_success", |b| {
            b.iter(|| {
                let current = db.get(&key).unwrap().unwrap();
                let new_val = match current.value {
                    Value::I64(n) => n + 1,
                    _ => 1,
                };
                let result = db.cas(run_id, key.clone(), current.version, Value::I64(new_val));
                black_box(result.unwrap());
            });
        });
    }

    // CAS failure (wrong version)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);
        let key = make_key(&ns, "cas_fail_key");
        db.put(run_id, key.clone(), Value::I64(0)).unwrap();

        group.bench_function("failure_wrong_version", |b| {
            b.iter(|| {
                // Always use wrong version
                let result = db.cas(run_id, key.clone(), 999999, Value::I64(1));
                black_box(result.is_err());
            });
        });
    }

    // CAS create (version 0 = insert if not exists)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_function("create_new_key", |b| {
            let counter = AtomicU64::new(0);
            b.iter(|| {
                let i = counter.fetch_add(1, Ordering::Relaxed);
                let key = make_key(&ns, &format!("cas_new_{}", i));
                let result = db.cas(run_id, key, 0, Value::I64(i as i64));
                black_box(result.unwrap());
            });
        });
    }

    group.finish();
}

// =============================================================================
// Snapshot Read Performance
// =============================================================================

fn snapshot_read_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_snapshot_read");
    group.throughput(Throughput::Elements(1));

    // Read within transaction (snapshot consistency)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        // Pre-populate
        for i in 0..1000 {
            let key = make_key(&ns, &format!("snap_key_{}", i));
            db.put(run_id, key, Value::I64(i)).unwrap();
        }

        group.bench_function("single_read_in_txn", |b| {
            let key = make_key(&ns, "snap_key_500");
            b.iter(|| {
                let result = db.transaction(run_id, |txn| txn.get(&key));
                black_box(result.unwrap());
            });
        });

        group.bench_function("multi_read_in_txn", |b| {
            let keys: Vec<_> = (0..10)
                .map(|i| make_key(&ns, &format!("snap_key_{}", i * 100)))
                .collect();

            b.iter(|| {
                let result = db.transaction(run_id, |txn| {
                    for key in &keys {
                        txn.get(key)?;
                    }
                    Ok(())
                });
                black_box(result.unwrap());
            });
        });
    }

    // Read-your-writes within transaction
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_function("read_your_writes", |b| {
            let counter = AtomicU64::new(0);
            b.iter(|| {
                let i = counter.fetch_add(1, Ordering::Relaxed);
                let key = make_key(&ns, &format!("ryw_{}", i));
                let result = db.transaction(run_id, |txn| {
                    txn.put(key.clone(), Value::I64(i as i64))?;
                    let val = txn.get(&key)?;
                    Ok(val)
                });
                black_box(result.unwrap());
            });
        });
    }

    group.finish();
}

// =============================================================================
// Conflict Detection (Multi-Threaded)
// =============================================================================

fn conflict_detection_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_conflict_detection");
    group.sample_size(30);

    // No contention (different keys per thread)
    for num_threads in [2, 4, 8] {
        group.throughput(Throughput::Elements(num_threads as u64));
        group.bench_with_input(
            BenchmarkId::new("no_contention_threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter_custom(|iters| {
                    let temp_dir = TempDir::new().unwrap();
                    let db = Arc::new(Database::open(temp_dir.path().join("db")).unwrap());
                    let run_id = RunId::new();

                    let barrier = Arc::new(Barrier::new(num_threads + 1));
                    let ops_per_thread = iters / num_threads as u64;

                    let handles: Vec<_> = (0..num_threads)
                        .map(|thread_id| {
                            let db = Arc::clone(&db);
                            let barrier = Arc::clone(&barrier);
                            let ns = create_namespace(run_id);

                            thread::spawn(move || {
                                barrier.wait();
                                for i in 0..ops_per_thread {
                                    let key = make_key(&ns, &format!("t{}_{}", thread_id, i));
                                    db.transaction(run_id, |txn| {
                                        txn.put(key.clone(), Value::I64(i as i64))?;
                                        Ok(())
                                    })
                                    .unwrap();
                                }
                            })
                        })
                        .collect();

                    let start = Instant::now();
                    barrier.wait();

                    for h in handles {
                        h.join().unwrap();
                    }

                    start.elapsed()
                });
            },
        );
    }

    // High contention (same key, all threads)
    for num_threads in [2, 4] {
        group.throughput(Throughput::Elements(num_threads as u64));
        group.bench_with_input(
            BenchmarkId::new("high_contention_threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter_custom(|iters| {
                    let temp_dir = TempDir::new().unwrap();
                    let db = Arc::new(Database::open(temp_dir.path().join("db")).unwrap());
                    let run_id = RunId::new();
                    let ns = create_namespace(run_id);
                    let contested_key = make_key(&ns, "contested");

                    db.put(run_id, contested_key.clone(), Value::I64(0)).unwrap();

                    let barrier = Arc::new(Barrier::new(num_threads + 1));
                    let ops_per_thread = iters / num_threads as u64;

                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let db = Arc::clone(&db);
                            let barrier = Arc::clone(&barrier);
                            let key = contested_key.clone();

                            thread::spawn(move || {
                                barrier.wait();
                                for _ in 0..ops_per_thread {
                                    // Retry on conflict
                                    loop {
                                        let result = db.transaction(run_id, |txn| {
                                            let val = txn.get(&key)?;
                                            let n = match val {
                                                Some(Value::I64(n)) => n,
                                                _ => 0,
                                            };
                                            txn.put(key.clone(), Value::I64(n + 1))?;
                                            Ok(())
                                        });
                                        if result.is_ok() {
                                            break;
                                        }
                                        thread::sleep(Duration::from_micros(10));
                                    }
                                }
                            })
                        })
                        .collect();

                    let start = Instant::now();
                    barrier.wait();

                    for h in handles {
                        h.join().unwrap();
                    }

                    start.elapsed()
                });
            },
        );
    }

    // CAS under contention (exactly one winner)
    group.bench_function("cas_one_winner", |b| {
        b.iter_custom(|iters| {
            let mut total_elapsed = Duration::ZERO;

            for _ in 0..iters {
                let temp_dir = TempDir::new().unwrap();
                let db = Arc::new(Database::open(temp_dir.path().join("db")).unwrap());
                let run_id = RunId::new();
                let ns = create_namespace(run_id);
                let key = make_key(&ns, "cas_contest");

                db.put(run_id, key.clone(), Value::I64(0)).unwrap();
                let initial_version = db.get(&key).unwrap().unwrap().version;

                let num_threads: usize = 4;
                let barrier = Arc::new(Barrier::new(num_threads + 1));
                let winners = Arc::new(AtomicU64::new(0));

                let handles: Vec<_> = (0..num_threads)
                    .map(|id| {
                        let db = Arc::clone(&db);
                        let barrier = Arc::clone(&barrier);
                        let winners = Arc::clone(&winners);
                        let key = key.clone();

                        thread::spawn(move || {
                            barrier.wait();
                            let result = db.cas(run_id, key, initial_version, Value::I64(id as i64));
                            if result.is_ok() {
                                winners.fetch_add(1, Ordering::Relaxed);
                            }
                        })
                    })
                    .collect();

                let start = Instant::now();
                barrier.wait();

                for h in handles {
                    h.join().unwrap();
                }

                total_elapsed += start.elapsed();

                // Invariant: exactly one winner
                assert_eq!(winners.load(Ordering::Relaxed), 1);
            }

            total_elapsed
        });
    });

    group.finish();
}

// =============================================================================
// Version Growth Impact
// =============================================================================

fn version_growth_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_version_growth");
    group.sample_size(20);

    // Measure transaction overhead as version count grows
    // (Tests that snapshot creation doesn't degrade with history)
    for num_prior_versions in [100, 1000, 10000] {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        // Create version history by updating same key many times
        let key = make_key(&ns, "versioned_key");
        for i in 0..num_prior_versions {
            db.put(run_id, key.clone(), Value::I64(i as i64)).unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("txn_after_versions", num_prior_versions),
            &num_prior_versions,
            |b, _| {
                let counter = AtomicU64::new(num_prior_versions as u64);
                b.iter(|| {
                    let i = counter.fetch_add(1, Ordering::Relaxed);
                    let result = db.transaction(run_id, |txn| {
                        let _ = txn.get(&key)?;
                        txn.put(key.clone(), Value::I64(i as i64))?;
                        Ok(())
                    });
                    black_box(result.unwrap());
                });
            },
        );
    }

    // Measure snapshot read with many versions
    for num_keys in [1000, 10000] {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        // Create keys with varying version counts
        for i in 0..num_keys {
            let key = make_key(&ns, &format!("multi_ver_{}", i));
            // Update each key 5 times
            for v in 0..5 {
                db.put(run_id, key.clone(), Value::I64(v)).unwrap();
            }
        }

        let lookup_key = make_key(&ns, &format!("multi_ver_{}", num_keys / 2));

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("snapshot_read_versioned", num_keys),
            &num_keys,
            |b, _| {
                b.iter(|| {
                    let result = db.transaction(run_id, |txn| txn.get(&lookup_key));
                    black_box(result.unwrap());
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(
    name = m2_commit;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = transaction_commit_benchmarks, cas_benchmarks, snapshot_read_benchmarks
);

criterion_group!(
    name = m2_conflict;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(30);
    targets = conflict_detection_benchmarks
);

criterion_group!(
    name = m2_version;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);
    targets = version_growth_benchmarks
);

criterion_main!(m2_commit, m2_conflict, m2_version);
