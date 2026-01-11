//! M1 Integration Tests
//!
//! These tests validate the complete M1 foundation works end-to-end:
//! - Database initialization and recovery
//! - Run lifecycle (begin_run, end_run)
//! - WAL logging and replay
//! - Storage operations
//! - KV primitive facade
//!
//! Success = M1 Foundation Complete

use in_mem_core::types::RunId;
use in_mem_engine::Database;
use in_mem_primitives::KVStore;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Test: Write via KV → restart → read via KV → data restored
#[test]
fn test_end_to_end_write_restart_read() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("e2e_db");

    let run_id = RunId::new();

    // Phase 1: Write data
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        // Begin run with tags
        db.begin_run(
            run_id,
            vec![
                ("env".to_string(), "test".to_string()),
                ("agent".to_string(), "test-agent".to_string()),
            ],
        )
        .unwrap();

        // Write KV data
        kv.put(run_id, "greeting", b"Hello, World!".to_vec())
            .unwrap();
        kv.put(run_id, "count", b"42".to_vec()).unwrap();
        kv.put(run_id, "status", b"running".to_vec()).unwrap();

        // End run
        db.end_run(run_id).unwrap();

        // Ensure flushed
        db.flush().unwrap();
    }

    // Phase 2: Reopen and verify
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        // Verify KV data restored
        assert_eq!(
            kv.get(run_id, "greeting").unwrap().unwrap(),
            b"Hello, World!"
        );
        assert_eq!(kv.get(run_id, "count").unwrap().unwrap(), b"42");
        assert_eq!(kv.get(run_id, "status").unwrap().unwrap(), b"running");

        // Verify run metadata restored
        let metadata = db.get_run(run_id).unwrap().unwrap();
        assert_eq!(metadata.run_id, run_id);
        assert_eq!(metadata.status, "completed");
        assert_eq!(metadata.tags.len(), 2);
    }
}

/// Test: Multiple runs maintain isolation across restart
#[test]
fn test_multiple_runs_isolation_across_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("multi_run_db");

    let run1 = RunId::new();
    let run2 = RunId::new();
    let run3 = RunId::new();

    // Write data for 3 runs
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        // Run 1
        db.begin_run(run1, vec![("name".to_string(), "run1".to_string())])
            .unwrap();
        kv.put(run1, "key", b"run1_value".to_vec()).unwrap();
        kv.put(run1, "run1_specific", b"data1".to_vec()).unwrap();
        db.end_run(run1).unwrap();

        // Run 2
        db.begin_run(run2, vec![("name".to_string(), "run2".to_string())])
            .unwrap();
        kv.put(run2, "key", b"run2_value".to_vec()).unwrap();
        kv.put(run2, "run2_specific", b"data2".to_vec()).unwrap();
        db.end_run(run2).unwrap();

        // Run 3
        db.begin_run(run3, vec![("name".to_string(), "run3".to_string())])
            .unwrap();
        kv.put(run3, "key", b"run3_value".to_vec()).unwrap();
        kv.put(run3, "run3_specific", b"data3".to_vec()).unwrap();
        db.end_run(run3).unwrap();

        db.flush().unwrap();
    }

    // Reopen and verify isolation
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db);

        // Each run should see its own data
        assert_eq!(kv.get(run1, "key").unwrap().unwrap(), b"run1_value");
        assert_eq!(kv.get(run2, "key").unwrap().unwrap(), b"run2_value");
        assert_eq!(kv.get(run3, "key").unwrap().unwrap(), b"run3_value");

        // Run-specific keys should only be visible to their run
        assert!(kv.get(run1, "run1_specific").unwrap().is_some());
        assert!(kv.get(run1, "run2_specific").unwrap().is_none());

        assert!(kv.get(run2, "run2_specific").unwrap().is_some());
        assert!(kv.get(run2, "run3_specific").unwrap().is_none());

        assert!(kv.get(run3, "run3_specific").unwrap().is_some());
        assert!(kv.get(run3, "run1_specific").unwrap().is_none());
    }
}

/// Test: Large dataset (1000 keys) survives restart
#[test]
fn test_large_dataset_survives_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("large_db");

    let run_id = RunId::new();
    let entry_count = 1000;

    // Write 1000 entries
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        db.begin_run(run_id, vec![]).unwrap();

        for i in 0..entry_count {
            let key = format!("key_{:04}", i);
            let value = format!("value_{:04}", i);
            kv.put(run_id, key.as_bytes(), value.as_bytes().to_vec())
                .unwrap();
        }

        db.end_run(run_id).unwrap();
        db.flush().unwrap();
    }

    // Reopen and verify all entries
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db);

        for i in 0..entry_count {
            let key = format!("key_{:04}", i);
            let expected = format!("value_{:04}", i);

            let actual = kv.get(run_id, key.as_bytes()).unwrap();
            assert!(actual.is_some(), "Key {} not found after restart", key);
            assert_eq!(
                actual.unwrap(),
                expected.as_bytes(),
                "Mismatch at key_{:04}",
                i
            );
        }
    }
}

/// Test: TTL expiration works correctly across restart
/// Note: Uses 1-second TTL (minimum supported by second-level precision)
#[test]
fn test_ttl_across_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("ttl_db");

    let run_id = RunId::new();

    // Write with short and long TTL
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        db.begin_run(run_id, vec![]).unwrap();

        // Short TTL (1 second - minimum supported)
        kv.put_with_ttl(
            run_id,
            "short_lived",
            b"expires_soon".to_vec(),
            Duration::from_secs(1),
        )
        .unwrap();

        // Long TTL (won't expire during test)
        kv.put_with_ttl(
            run_id,
            "long_lived",
            b"persists".to_vec(),
            Duration::from_secs(3600),
        )
        .unwrap();

        db.end_run(run_id).unwrap();
        db.flush().unwrap();
    }

    // Wait for short TTL to expire
    std::thread::sleep(Duration::from_millis(1100));

    // Reopen and verify TTL behavior
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db);

        // Short-lived should be expired
        assert!(
            kv.get(run_id, "short_lived").unwrap().is_none(),
            "Short-lived key should have expired"
        );

        // Long-lived should still exist
        assert_eq!(
            kv.get(run_id, "long_lived").unwrap().unwrap(),
            b"persists",
            "Long-lived key should still exist"
        );
    }
}

/// Test: Run metadata fully persists and restores
#[test]
fn test_run_metadata_completeness() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("metadata_db");

    let run_id = RunId::new();

    // Create run with metadata
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        db.begin_run(
            run_id,
            vec![
                ("user".to_string(), "alice".to_string()),
                ("task".to_string(), "data_processing".to_string()),
                ("priority".to_string(), "high".to_string()),
            ],
        )
        .unwrap();

        // Do some work
        kv.put(run_id, "result", b"success".to_vec()).unwrap();

        db.end_run(run_id).unwrap();
        db.flush().unwrap();
    }

    // Reopen and verify metadata
    {
        let db = Database::open(&db_path).unwrap();
        let metadata = db.get_run(run_id).unwrap().unwrap();

        // Check all fields
        assert_eq!(metadata.run_id, run_id);
        assert_eq!(metadata.status, "completed");
        assert!(metadata.created_at > 0);
        assert!(metadata.completed_at.is_some());
        assert!(metadata.completed_at.unwrap() >= metadata.created_at);

        // Check tags
        assert_eq!(metadata.tags.len(), 3);
        assert!(metadata
            .tags
            .contains(&("user".to_string(), "alice".to_string())));
        assert!(metadata
            .tags
            .contains(&("task".to_string(), "data_processing".to_string())));
        assert!(metadata
            .tags
            .contains(&("priority".to_string(), "high".to_string())));
    }
}

/// Test: List operations work after recovery
#[test]
fn test_list_operations_across_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("list_db");

    let run_id = RunId::new();

    // Write data with prefixes
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        db.begin_run(run_id, vec![]).unwrap();

        kv.put(run_id, "user:alice:name", b"Alice".to_vec())
            .unwrap();
        kv.put(run_id, "user:alice:age", b"30".to_vec()).unwrap();
        kv.put(run_id, "user:bob:name", b"Bob".to_vec()).unwrap();
        kv.put(run_id, "user:bob:age", b"25".to_vec()).unwrap();
        kv.put(run_id, "config:timeout", b"60".to_vec()).unwrap();
        kv.put(run_id, "config:retries", b"3".to_vec()).unwrap();

        db.end_run(run_id).unwrap();
        db.flush().unwrap();
    }

    // Reopen and verify list operations
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db);

        // List all user: entries
        let user_entries = kv.list(run_id, "user:").unwrap();
        assert_eq!(user_entries.len(), 4, "Should have 4 user entries");

        // List alice entries
        let alice_entries = kv.list(run_id, "user:alice:").unwrap();
        assert_eq!(alice_entries.len(), 2, "Should have 2 alice entries");

        // List config entries
        let config_entries = kv.list(run_id, "config:").unwrap();
        assert_eq!(config_entries.len(), 2, "Should have 2 config entries");

        // Verify specific values
        let alice_name = kv.get(run_id, "user:alice:name").unwrap().unwrap();
        assert_eq!(alice_name, b"Alice");
    }
}

/// Test: Delete operations persist across restart
#[test]
fn test_delete_operations_across_restart() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("delete_db");

    let run_id = RunId::new();

    // Write and delete data
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        db.begin_run(run_id, vec![]).unwrap();

        // Write 3 keys
        kv.put(run_id, "keep1", b"value1".to_vec()).unwrap();
        kv.put(run_id, "delete_me", b"goodbye".to_vec()).unwrap();
        kv.put(run_id, "keep2", b"value2".to_vec()).unwrap();

        // Delete one
        let deleted = kv.delete(run_id, "delete_me").unwrap();
        assert_eq!(deleted.unwrap(), b"goodbye");

        db.end_run(run_id).unwrap();
        db.flush().unwrap();
    }

    // Reopen and verify deletion persisted
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db);

        // Kept keys should exist
        assert_eq!(kv.get(run_id, "keep1").unwrap().unwrap(), b"value1");
        assert_eq!(kv.get(run_id, "keep2").unwrap().unwrap(), b"value2");

        // Deleted key should be gone
        assert!(kv.get(run_id, "delete_me").unwrap().is_none());
    }
}

/// Test: Complete M1 workflow - the final validation test
#[test]
fn test_m1_complete_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("m1_complete");

    println!("=== M1 Integration Test: Complete Workflow ===");

    let run_id = RunId::new();

    // Phase 1: Initialize and write
    println!("Phase 1: Writing data...");
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        // Begin run
        db.begin_run(
            run_id,
            vec![("test".to_string(), "m1_integration".to_string())],
        )
        .unwrap();

        // Write various data
        kv.put(run_id, "agent_state", b"initialized".to_vec())
            .unwrap();
        kv.put(run_id, "tool_output", b"weather: 72F".to_vec())
            .unwrap();
        kv.put(run_id, "decision", b"umbrella_not_needed".to_vec())
            .unwrap();

        // Update state
        kv.put(run_id, "agent_state", b"completed".to_vec())
            .unwrap();

        // End run
        db.end_run(run_id).unwrap();
        db.flush().unwrap();

        println!("  Wrote 3 KV pairs, ended run");
    }

    // Phase 2: Simulate crash/restart
    println!("Phase 2: Simulating restart...");

    // Phase 3: Recover and verify
    println!("Phase 3: Recovering...");
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        println!("  Recovery complete");

        // Verify data
        assert_eq!(
            kv.get(run_id, "agent_state").unwrap().unwrap(),
            b"completed",
            "Agent state should be 'completed'"
        );
        assert_eq!(
            kv.get(run_id, "tool_output").unwrap().unwrap(),
            b"weather: 72F",
            "Tool output should be preserved"
        );
        assert_eq!(
            kv.get(run_id, "decision").unwrap().unwrap(),
            b"umbrella_not_needed",
            "Decision should be preserved"
        );

        println!("  All data verified");

        // Verify run metadata
        let metadata = db.get_run(run_id).unwrap().unwrap();
        assert_eq!(metadata.status, "completed");
        assert!(metadata
            .tags
            .contains(&("test".to_string(), "m1_integration".to_string())));

        println!("  Run metadata verified");
    }

    println!("=== M1 Integration Test: PASSED ===");
}

/// Test: Concurrent writes from multiple KVStore instances
#[test]
fn test_concurrent_kv_writes() {
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("concurrent_db");

    let db = Arc::new(Database::open(&db_path).unwrap());
    let run_id = RunId::new();

    db.begin_run(run_id, vec![]).unwrap();

    let mut handles = vec![];

    // Spawn 10 threads, each writing 100 keys
    for thread_id in 0..10 {
        let kv = KVStore::new(db.clone());
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let key = format!("thread_{}_key_{}", thread_id, i);
                let value = format!("thread_{}_value_{}", thread_id, i);
                kv.put(run_id, key.as_bytes(), value.as_bytes().to_vec())
                    .unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    db.end_run(run_id).unwrap();
    db.flush().unwrap();

    // Drop and reopen
    drop(db);

    let db = Arc::new(Database::open(&db_path).unwrap());
    let kv = KVStore::new(db);

    // Verify all data
    for thread_id in 0..10 {
        for i in 0..100 {
            let key = format!("thread_{}_key_{}", thread_id, i);
            let expected = format!("thread_{}_value_{}", thread_id, i);
            let actual = kv.get(run_id, key.as_bytes()).unwrap().unwrap();
            assert_eq!(actual, expected.as_bytes());
        }
    }
}

/// Test: Multiple sequential restarts
///
/// Tests that data written in session N survives restart and is readable in session N+1.
/// Uses separate runs for each session to avoid run tracking complexity.
#[test]
fn test_multiple_restarts() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("restart_db");

    // Use separate runs for each session
    let run1 = RunId::new();
    let run2 = RunId::new();
    let run3 = RunId::new();

    // First session
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());
        db.begin_run(run1, vec![("session".to_string(), "1".to_string())])
            .unwrap();
        kv.put(run1, "key", b"data1".to_vec()).unwrap();
        db.end_run(run1).unwrap();
        db.flush().unwrap();
    }

    // Second session - verify previous and add more
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());
        // Verify previous data
        assert_eq!(kv.get(run1, "key").unwrap().unwrap(), b"data1");
        // Add more in a new run
        db.begin_run(run2, vec![("session".to_string(), "2".to_string())])
            .unwrap();
        kv.put(run2, "key", b"data2".to_vec()).unwrap();
        db.end_run(run2).unwrap();
        db.flush().unwrap();
    }

    // Third session - verify all previous and add more
    {
        // Debug: Check WAL entries before opening database
        use in_mem_durability::wal::{DurabilityMode, WAL};
        let wal = WAL::open(db_path.join("wal/current.wal"), DurabilityMode::Strict).unwrap();
        let entries = wal.read_all().unwrap();
        println!("DEBUG: Before session 3, WAL has {} entries", entries.len());
        drop(wal);

        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());

        // Debug: Check what's recovered
        let run1_data = kv.get(run1, "key").unwrap();
        let run2_data = kv.get(run2, "key").unwrap();
        println!(
            "DEBUG: run1 data: {:?}, run2 data: {:?}",
            run1_data.as_ref().map(|d| String::from_utf8_lossy(d)),
            run2_data.as_ref().map(|d| String::from_utf8_lossy(d))
        );

        // Verify all previous data
        assert!(run1_data.is_some(), "run1 data should exist");
        assert_eq!(run1_data.unwrap(), b"data1");
        assert!(run2_data.is_some(), "run2 data should exist");
        assert_eq!(run2_data.unwrap(), b"data2");
        // Add more in a new run
        db.begin_run(run3, vec![("session".to_string(), "3".to_string())])
            .unwrap();
        kv.put(run3, "key", b"data3".to_vec()).unwrap();
        db.end_run(run3).unwrap();
        db.flush().unwrap();
    }

    // Final verification
    {
        let db = Arc::new(Database::open(&db_path).unwrap());
        let kv = KVStore::new(db.clone());
        assert_eq!(kv.get(run1, "key").unwrap().unwrap(), b"data1");
        assert_eq!(kv.get(run2, "key").unwrap().unwrap(), b"data2");
        assert_eq!(kv.get(run3, "key").unwrap().unwrap(), b"data3");

        // Verify all runs completed
        assert_eq!(db.get_run(run1).unwrap().unwrap().status, "completed");
        assert_eq!(db.get_run(run2).unwrap().unwrap().status, "completed");
        assert_eq!(db.get_run(run3).unwrap().unwrap().status, "completed");
    }
}
