# Commit Protocol Consolidation

## Status: Implemented

## Problem

The engine crate's `Database::commit_internal()` reimplements the commit protocol
that the concurrency crate's `TransactionManager::commit()` already provides. This
is an architectural violation: the concurrency layer exists specifically to ensure
safe multi-threaded database mutations, and the engine bypasses it entirely.

### Evidence of Duplication

Both implementations perform the identical sequence:

| Step | `Database::commit_internal()` | `TransactionManager::commit()` |
|------|-------------------------------|--------------------------------|
| 1. Per-run lock | `self.commit_locks` (DashMap on Database) | `self.commit_locks` (DashMap on TransactionManager) |
| 2. Validate | `txn.mark_validating()` + `validate_transaction()` | `txn.commit(store)` (combines validate + mark) |
| 3. Version | `self.coordinator.allocate_commit_version()` | `self.allocate_version()` |
| 4. WAL write | Manual `TransactionWALWriter` usage | Manual `TransactionWALWriter` usage |
| 5. Storage apply | `self.storage.apply_batch()` | `txn.apply_writes(store, version)` |
| 6. Mark committed | `txn.mark_committed()` | Already done in step 2 |

Specific duplications:

1. **Per-run commit locks**: Both layers maintain independent `DashMap<RunId, Mutex<()>>`
   for serializing commits within a run. Two sets of locks exist for the same purpose.

2. **WAL writing**: Both layers manually construct `TransactionWALWriter`, call
   `write_begin()`, iterate write/delete/CAS sets, and call `write_commit()`.

3. **Validation**: The engine imports and calls `validate_transaction()` directly,
   duplicating what `TransactionContext::commit()` already does internally.

4. **Storage application**: The engine calls `self.storage.apply_batch()` directly,
   while TransactionManager calls `txn.apply_writes(store, version)` which does
   the same thing through the Storage trait.

### Why This Happened

The engine reimplemented the protocol for three practical reasons:

1. **Ephemeral databases**: `TransactionManager::commit()` required a `&WAL`
   parameter. Ephemeral databases have no WAL (`wal: None`).

2. **Durability modes**: The engine conditionally skips WAL writes when
   `durability.requires_wal()` is false. TransactionManager always wrote to WAL.

3. **Error types**: TransactionManager returns `Result<u64, CommitError>`,
   while the engine expects `StrataResult<u64>`.

### Consequences

- The concurrency layer's `TransactionManager::commit()` is effectively dead code,
  despite being the architecturally correct owner of the commit protocol.
- The concurrency layer's comprehensive tests for commit serialization, TOCTOU
  prevention, and crash scenarios validate code that isn't actually used in production.
- Bug fixes to the commit protocol must be applied in two places.
- The engine maintains its own `commit_locks: DashMap<RunId, ParkingMutex<()>>`,
  while TransactionManager has an identical but unused `commit_locks: DashMap<RunId, Mutex<()>>`.

## Solution

### Design Principle

**The concurrency layer owns the commit protocol.** The engine delegates to it.

The engine is responsible for:
- Managing WAL lifecycle (open/close, Mutex wrapping)
- Deciding whether to pass the WAL based on durability mode
- Handling fsync for Strict durability mode
- Recording metrics (commit/abort counts)

The concurrency layer is responsible for:
- Per-run commit locking (TOCTOU prevention)
- Validation (first-committer-wins)
- Version allocation
- WAL writing (when WAL is provided)
- Storage application

### Changes

#### 1. TransactionManager::commit() accepts Optional WAL

```rust
// Before
pub fn commit<S: Storage>(
    &self,
    txn: &mut TransactionContext,
    store: &S,
    wal: &WAL,
) -> Result<u64, CommitError>

// After
pub fn commit<S: Storage>(
    &self,
    txn: &mut TransactionContext,
    store: &S,
    wal: Option<&WAL>,
) -> Result<u64, CommitError>
```

When `wal` is `None`, the commit skips WAL writes entirely. Validation, version
allocation, and storage application still execute. This supports ephemeral databases
and `DurabilityMode::None`.

#### 2. TransactionCoordinator gains a commit() method

```rust
impl TransactionCoordinator {
    pub fn commit<S: Storage>(
        &self,
        txn: &mut TransactionContext,
        store: &S,
        wal: Option<&WAL>,
    ) -> StrataResult<u64> {
        match self.manager.commit(txn, store, wal) {
            Ok(version) => {
                self.record_commit();
                Ok(version)
            }
            Err(e) => {
                self.record_abort();
                Err(StrataError::from(e))
            }
        }
    }
}
```

This method:
- Delegates the commit protocol to TransactionManager
- Records metrics (commit/abort)
- Converts `CommitError` to `StrataError`

#### 3. Database::commit_internal() delegates

```rust
fn commit_internal(
    &self,
    txn: &mut TransactionContext,
    durability: DurabilityMode,
) -> StrataResult<u64> {
    // Determine WAL reference based on durability mode and persistence
    let wal_guard = if durability.requires_wal() {
        self.wal.as_ref().map(|w| w.lock())
    } else {
        None
    };
    let wal_ref = wal_guard.as_deref();

    // Delegate to concurrency layer
    let version = self.coordinator.commit(txn, self.storage.as_ref(), wal_ref)?;

    // Strict mode: fsync after commit
    if durability.requires_immediate_fsync() {
        if let Some(ref guard) = wal_guard {
            guard.fsync()?;
        }
    }

    Ok(version)
}
```

#### 4. Removed from Database

- `commit_locks: DashMap<RunId, ParkingMutex<()>>` field
- All manual WAL writing code in `commit_internal()`
- The `validate_transaction` import (no longer used directly)
- The `TransactionWALWriter` import (no longer used directly)

### Additional Fix: WAL Transaction ID Consistency

The original `TransactionManager::commit()` allocated a *separate* txn_id for the
WAL writer via `self.next_txn_id()`, different from the TransactionContext's own
`txn.txn_id`. This meant the WAL recorded a different transaction ID than the one
the TransactionContext was created with. The engine's implementation correctly used
`txn.txn_id`. The fix uses `txn.txn_id` consistently.

### Note: WAL Internal Synchronization

The WAL struct uses `Arc<Mutex<BufWriter<File>>>` internally for thread-safe
appends. This means the engine's `ParkingMutex<WAL>` wrapper provides redundant
synchronization. However, removing it is a separate concern and is not addressed
in this change. The redundant lock is harmless (adds negligible overhead since WAL
writes already serialize internally).

## Files Modified

| File | Change |
|------|--------|
| `crates/concurrency/src/manager.rs` | `commit()` accepts `Option<&WAL>`, uses `txn.txn_id` |
| `crates/engine/src/coordinator.rs` | Added `commit()` method |
| `crates/engine/src/database/mod.rs` | Removed `commit_locks`, simplified `commit_internal()` |

## Verification

```bash
cargo test -p strata-concurrency
cargo test -p strata-engine
cargo test  # Full workspace
```
