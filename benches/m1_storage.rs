//! M1 Storage Benchmarks
//!
//! These benchmarks focus on M1 (Storage + WAL) primitives only.
//! No transactions, no isolation, no concurrency guarantees.
//!
//! ## What M1 is:
//! - Single-threaded correctness
//! - Deterministic state rebuild
//! - Append-only WAL
//!
//! ## Target Performance (MVP)
//!
//! | Operation     | Stretch Goal   | Acceptable     | Notes                    |
//! |---------------|----------------|----------------|--------------------------|
//! | KV get        | 50-200K ops/s  | 10-40K ops/s   | RwLock + BTreeMap        |
//! | KV put        | 10-50K ops/s   | 5-20K ops/s    | + WAL append             |
//! | WAL append    | 100K+ ops/s    | 20K+ ops/s     | Buffered writes          |
//! | WAL replay    | <300ms/1M ops  | <1s/1M ops     | Cold start               |
//!
//! **These are stretch goals. MVP success is semantic correctness first,
//! performance second.**
//!
//! ## Running
//!
//! ```bash
//! cargo bench --bench m1_storage
//! ```

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use in_mem_core::types::{Key, Namespace, RunId};
use in_mem_core::value::Value;
use in_mem_engine::Database;
use std::sync::atomic::{AtomicU64, Ordering};
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
// KV Get Throughput
// =============================================================================

fn kv_get_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_kv_get");

    // Pre-populate database
    let temp_dir = TempDir::new().unwrap();
    let db = Database::open(temp_dir.path().join("db")).unwrap();
    let run_id = RunId::new();
    let ns = create_namespace(run_id);

    for i in 0..10_000 {
        let key = make_key(&ns, &format!("key_{:05}", i));
        db.put(run_id, key, Value::I64(i)).unwrap();
    }

    group.throughput(Throughput::Elements(1));

    // Get existing key (hot path)
    group.bench_function("existing_key", |b| {
        let key = make_key(&ns, "key_05000");
        b.iter(|| {
            let result = db.get(&key);
            black_box(result.unwrap());
        });
    });

    // Get nonexistent key (miss path)
    group.bench_function("nonexistent_key", |b| {
        let key = make_key(&ns, "nonexistent");
        b.iter(|| {
            let result = db.get(&key);
            black_box(result.unwrap());
        });
    });

    // Get with varying key positions (early, middle, late in BTreeMap)
    for position in ["early", "middle", "late"] {
        let key_name = match position {
            "early" => "key_00100",
            "middle" => "key_05000",
            "late" => "key_09900",
            _ => unreachable!(),
        };
        let key = make_key(&ns, key_name);

        group.bench_with_input(
            BenchmarkId::new("position", position),
            &position,
            |b, _| {
                b.iter(|| {
                    let result = db.get(&key);
                    black_box(result.unwrap());
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// KV Put Throughput
// =============================================================================

fn kv_put_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_kv_put");
    group.throughput(Throughput::Elements(1));

    // Put unique keys (append pattern)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_function("unique_keys", |b| {
            let counter = AtomicU64::new(0);
            b.iter(|| {
                let i = counter.fetch_add(1, Ordering::Relaxed);
                let key = make_key(&ns, &format!("unique_{}", i));
                let result = db.put(run_id, key, Value::I64(i as i64));
                black_box(result.unwrap());
            });
        });
    }

    // Put overwrite (update pattern)
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);
        let key = make_key(&ns, "overwrite_key");
        db.put(run_id, key.clone(), Value::I64(0)).unwrap();

        group.bench_function("overwrite_same_key", |b| {
            let counter = AtomicU64::new(0);
            b.iter(|| {
                let i = counter.fetch_add(1, Ordering::Relaxed);
                let result = db.put(run_id, key.clone(), Value::I64(i as i64));
                black_box(result.unwrap());
            });
        });
    }

    // Delete throughput
    {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        group.bench_function("delete", |b| {
            let counter = AtomicU64::new(0);
            b.iter_custom(|iters| {
                // Setup: create keys
                let start_idx = counter.fetch_add(iters, Ordering::Relaxed);
                for i in start_idx..(start_idx + iters) {
                    let key = make_key(&ns, &format!("del_{}", i));
                    db.put(run_id, key, Value::I64(i as i64)).unwrap();
                }

                // Benchmark: delete
                let start = Instant::now();
                for i in start_idx..(start_idx + iters) {
                    let key = make_key(&ns, &format!("del_{}", i));
                    db.delete(run_id, key).unwrap();
                }
                start.elapsed()
            });
        });
    }

    group.finish();
}

// =============================================================================
// Value Size Impact
// =============================================================================

fn value_size_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_value_size");

    for value_size in [64, 256, 1024, 4096, 16384] {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);
        let data = vec![0u8; value_size];

        group.throughput(Throughput::Bytes(value_size as u64));
        group.bench_with_input(
            BenchmarkId::new("put_bytes", value_size),
            &value_size,
            |b, _| {
                let counter = AtomicU64::new(0);
                b.iter(|| {
                    let i = counter.fetch_add(1, Ordering::Relaxed);
                    let key = make_key(&ns, &format!("sized_{}", i));
                    let result = db.put(run_id, key, Value::Bytes(data.clone()));
                    black_box(result.unwrap());
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// WAL Replay Performance
// =============================================================================

fn wal_replay_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_wal_replay");
    group.sample_size(20); // Fewer samples, these are slow

    for num_ops in [1_000, 10_000, 50_000] {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");

        // Setup: populate database
        {
            let db = Database::open(&db_path).unwrap();
            let run_id = RunId::new();
            let ns = create_namespace(run_id);

            for i in 0..num_ops {
                let key = make_key(&ns, &format!("key_{}", i));
                db.put(run_id, key, Value::I64(i as i64)).unwrap();
            }
            db.flush().unwrap();
        }

        group.throughput(Throughput::Elements(num_ops as u64));
        group.bench_with_input(
            BenchmarkId::new("replay_ops", num_ops),
            &num_ops,
            |b, _| {
                b.iter(|| {
                    // Re-open triggers recovery
                    let db = Database::open(&db_path).unwrap();
                    black_box(db);
                });
            },
        );
    }

    // Mixed workload replay (puts + deletes)
    {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let num_ops = 10_000;

        {
            let db = Database::open(&db_path).unwrap();
            let run_id = RunId::new();
            let ns = create_namespace(run_id);

            for i in 0..num_ops {
                let key = make_key(&ns, &format!("mixed_{}", i));
                db.put(run_id, key, Value::I64(i as i64)).unwrap();
            }
            // Delete every 10th key
            for i in (0..num_ops).step_by(10) {
                let key = make_key(&ns, &format!("mixed_{}", i));
                db.delete(run_id, key).unwrap();
            }
            db.flush().unwrap();
        }

        group.throughput(Throughput::Elements(num_ops as u64));
        group.bench_function("replay_mixed_workload", |b| {
            b.iter(|| {
                let db = Database::open(&db_path).unwrap();
                black_box(db);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Memory Overhead (Key Count Scaling)
// =============================================================================

fn memory_overhead_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_memory_overhead");
    group.sample_size(10);

    // Measure get latency as key count increases
    // (This indirectly measures BTreeMap lookup scaling)
    for num_keys in [1_000, 10_000, 100_000] {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::open(temp_dir.path().join("db")).unwrap();
        let run_id = RunId::new();
        let ns = create_namespace(run_id);

        // Populate
        for i in 0..num_keys {
            let key = make_key(&ns, &format!("key_{:06}", i));
            db.put(run_id, key, Value::I64(i as i64)).unwrap();
        }

        let lookup_key = make_key(&ns, &format!("key_{:06}", num_keys / 2));

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("get_at_scale", num_keys),
            &num_keys,
            |b, _| {
                b.iter(|| {
                    let result = db.get(&lookup_key);
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
    name = m1_kv;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = kv_get_benchmarks, kv_put_benchmarks, value_size_benchmarks
);

criterion_group!(
    name = m1_wal;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(20);
    targets = wal_replay_benchmarks
);

criterion_group!(
    name = m1_memory;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(10);
    targets = memory_overhead_benchmarks
);

criterion_main!(m1_kv, m1_wal, m1_memory);
