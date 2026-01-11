# Epic 2: Storage Layer - Claude Prompts

**Epic Branch**: `epic-2-storage-layer`

**Dependencies**: Epic 1 must be complete and merged to develop ✅

---

## Parallelization Strategy

### Phase 1: Foundation (Sequential) - Story #12
**Story #12 (UnifiedStore)** must complete first - it implements the Storage trait and blocks all other stories.

**Estimated**: 5-6 hours

### Phase 2: Indices and Snapshot (3 Claudes in Parallel) - Stories #13, #14, #15
After #12 merges to epic branch, these can run in parallel:
- Story #13: Secondary indices (run_index, type_index)
- Story #14: TTL index and cleanup
- Story #15: ClonedSnapshotView implementation

**Estimated**: 4-5 hours wall time (parallel)

### Phase 3: Comprehensive Testing (Sequential) - Story #16
After #13-15 merge, final story adds comprehensive storage tests.

**Estimated**: 3-4 hours

**Total Epic 2**: ~13-15 hours sequential, ~9-11 hours with 3 Claudes in parallel

---

## Prompt 1: Story #12 - UnifiedStore (MUST DO FIRST)

### Context
You are implementing Story #12 for the in-mem database project. Epic 1 (Workspace & Core Types) is complete with Storage and SnapshotView traits defined. You are now implementing the MVP storage backend.

### Your Task
Implement UnifiedStore with BTreeMap backend and version management in the `in-mem-storage` crate.

### Prerequisites
1. Clone the repository:
   ```bash
   git clone https://github.com/anibjoshi/in-mem.git
   cd in-mem
   ```

2. Checkout develop and create your story branch:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b epic-2-story-12-unified-store
   ```

3. Read the following for context:
   - `docs/architecture/M1_ARCHITECTURE.md` - Complete M1 specification
   - `docs/development/TDD_METHODOLOGY.md` - Testing approach
   - `docs/development/GETTING_STARTED.md` - Development workflow
   - `crates/core/src/traits.rs` - Storage trait you're implementing
   - `crates/core/src/types.rs` - Key, RunId, Namespace, TypeTag types
   - `crates/core/src/value.rs` - Value and VersionedValue types

4. View the GitHub issue for complete requirements:
   ```bash
   gh issue view 12
   ```

### Implementation Steps

#### Step 1: Update Cargo.toml
Add `parking_lot` dependency to `crates/storage/Cargo.toml`:
```toml
[dependencies]
in-mem-core = { path = "../core" }
parking_lot = "0.12"
```

#### Step 2: Create unified.rs
Implement `crates/storage/src/unified.rs` with:
- `UnifiedStore` struct with `Arc<RwLock<BTreeMap<Key, VersionedValue>>>`
- `AtomicU64` for global version counter
- Implement all Storage trait methods:
  - `get()` - filters out expired values
  - `get_versioned()` - respects max_version and expiration
  - `put()` - assigns next version atomically
  - `delete()` - removes key
  - `scan_prefix()` - BTreeMap range scan
  - `scan_by_run()` - filter by namespace.run_id
  - `current_version()` - returns last assigned version
  - `find_expired_keys()` - finds TTL-expired keys

#### Step 3: Write Tests First (TDD)
Write these tests BEFORE implementing (in `unified.rs` #[cfg(test)] module):
1. `test_store_creation` - empty store, current_version=0
2. `test_put_and_get` - basic write and read
3. `test_version_monotonicity` - versions increase 1,2,3...
4. `test_get_versioned` - respects max_version parameter
5. `test_delete` - removes key correctly
6. `test_ttl_expiration` - expired values return None
7. `test_scan_prefix` - BTreeMap range query
8. `test_scan_by_run` - filters by run_id
9. `test_concurrent_writes` - 10 threads × 100 writes = 1000 versions

#### Step 4: Implement to Pass Tests
Implement UnifiedStore methods to make all tests pass. Key points:
- Version allocation BEFORE acquiring write lock (allocate first, write second)
- TTL check in `is_expired()` helper method
- `scan_prefix` uses BTreeMap `.range(prefix..)` with `.take_while()`
- `scan_by_run` filters by `key.namespace.run_id`

#### Step 5: Update lib.rs
Update `crates/storage/src/lib.rs`:
```rust
pub mod unified;

pub use unified::UnifiedStore;
```

#### Step 6: Verify
```bash
# Build
cargo build -p in-mem-storage

# Test
cargo test -p in-mem-storage

# Clippy
cargo clippy -p in-mem-storage -- -D warnings

# Format
cargo fmt -p in-mem-storage
```

### Acceptance Criteria
- [ ] All 9 unit tests pass
- [ ] Version numbers are monotonically increasing (1, 2, 3, ...)
- [ ] TTL expiration works correctly
- [ ] scan_prefix returns only matching keys
- [ ] scan_by_run filters by run_id
- [ ] Concurrent writes work (test with 10 threads)
- [ ] 100% test coverage for unified.rs
- [ ] No clippy warnings
- [ ] All code formatted with cargo fmt

### When Complete
1. Run the complete-story script:
   ```bash
   ./scripts/complete-story.sh 12
   ```

2. Comment on GitHub issue #12 with your PR link

3. **IMPORTANT**: Notify other Claudes that Story #12 is complete so they can start #13, #14, #15 in parallel

### Notes
- **Known limitation**: This implementation overwrites old versions (no version history). Acceptable for MVP.
- **Known bottleneck**: RwLock will contend under high concurrency. Storage trait allows future replacement.
- Use `parking_lot::RwLock` instead of `std::sync::RwLock` (more efficient)
- TTL expiration is logical (values filtered at read time, not physically deleted)

### Questions?
- Check `docs/architecture/M1_ARCHITECTURE.md` Section 6 (Storage Layer)
- Check `docs/development/TDD_METHODOLOGY.md` for testing approach
- Ask in GitHub issue #12 comments if blocked

### Repository
https://github.com/anibjoshi/in-mem

---

## Prompt 2: Story #13 - Secondary Indices (WAIT FOR #12)

### Context
You are implementing Story #13 for the in-mem database project. Story #12 (UnifiedStore) is complete. You are now adding secondary indices for efficient queries.

### Your Task
Add run_index and type_index secondary indices to UnifiedStore for efficient run-scoped and type-scoped queries.

### Prerequisites
1. **WAIT FOR STORY #12 TO MERGE** to `epic-2-storage-layer` branch

2. Once #12 is merged, pull and create your branch:
   ```bash
   git checkout epic-2-storage-layer
   git pull origin epic-2-storage-layer
   git checkout -b epic-2-story-13-secondary-indices
   ```

3. Read context:
   - `docs/architecture/M1_ARCHITECTURE.md` - Storage layer design
   - `crates/storage/src/unified.rs` - UnifiedStore implementation (from #12)
   - `crates/core/src/types.rs` - RunId, TypeTag types

4. View the GitHub issue:
   ```bash
   gh issue view 13
   ```

### Implementation Steps

#### Step 1: Create index.rs
Create `crates/storage/src/index.rs` with:
- `RunIndex` - maps RunId → HashSet<Key>
- `TypeIndex` - maps TypeTag → HashSet<Key>
- Methods: `insert()`, `remove()`, `get_keys()`

#### Step 2: Modify UnifiedStore
Update `crates/storage/src/unified.rs` to:
- Add `run_index: RunIndex` field
- Add `type_index: TypeIndex` field
- Update `put()` to insert into both indices
- Update `delete()` to remove from both indices
- Update `scan_by_run()` to use run_index (much faster)
- Add `scan_by_type()` method using type_index

#### Step 3: Write Tests First (TDD)
Add tests to `index.rs`:
1. `test_run_index_insert_and_get`
2. `test_run_index_remove`
3. `test_type_index_insert_and_get`
4. `test_type_index_remove`

Add tests to `unified.rs`:
5. `test_scan_by_run_uses_index` - verify faster lookup
6. `test_scan_by_type` - all events, all KV entries, etc.
7. `test_indices_stay_consistent` - put+delete keeps indices in sync

#### Step 4: Implement to Pass Tests
Implement index structures and update UnifiedStore methods.

**Key point**: All index updates must happen within the same write lock as the main BTreeMap (atomic updates).

#### Step 5: Update lib.rs
Update `crates/storage/src/lib.rs`:
```rust
pub mod unified;
pub mod index;

pub use unified::UnifiedStore;
pub use index::{RunIndex, TypeIndex};
```

#### Step 6: Verify
```bash
cargo test -p in-mem-storage
cargo clippy -p in-mem-storage -- -D warnings
cargo fmt -p in-mem-storage
```

### Acceptance Criteria
- [ ] RunIndex and TypeIndex implemented
- [ ] UnifiedStore.put() updates both indices
- [ ] UnifiedStore.delete() removes from both indices
- [ ] scan_by_run() uses run_index (O(run size) not O(total data))
- [ ] scan_by_type() implemented and works
- [ ] All 7 tests pass
- [ ] Indices stay consistent (verified by test)

### When Complete
```bash
./scripts/complete-story.sh 13
```

### Notes
- **Critical**: Index updates must be atomic with main storage (same write lock)
- Use `HashMap<RunId, HashSet<Key>>` for run_index
- Use `HashMap<TypeTag, HashSet<Key>>` for type_index

### Repository
https://github.com/anibjoshi/in-mem

---

## Prompt 3: Story #14 - TTL Index (WAIT FOR #12)

### Context
You are implementing Story #14 for the in-mem database project. Story #12 (UnifiedStore) is complete. You are now adding TTL index and cleanup subsystem.

### Your Task
Add TTL index to UnifiedStore for efficient TTL expiration cleanup.

### Prerequisites
1. **WAIT FOR STORY #12 TO MERGE** to `epic-2-storage-layer` branch

2. Once #12 is merged, pull and create your branch:
   ```bash
   git checkout epic-2-storage-layer
   git pull origin epic-2-storage-layer
   git checkout -b epic-2-story-14-ttl-index
   ```

3. Read context:
   - `docs/architecture/M1_ARCHITECTURE.md` - TTL cleanup design
   - `crates/storage/src/unified.rs` - UnifiedStore implementation
   - GitHub issue #14 for complete requirements

### Implementation Steps

#### Step 1: Create ttl.rs
Create `crates/storage/src/ttl.rs` with:
- `TTLIndex` - maps Instant (expiry_time) → HashSet<Key>
- Methods: `insert()`, `remove()`, `find_expired()`
- Use `BTreeMap<Instant, HashSet<Key>>` for sorted expiry times

#### Step 2: Modify UnifiedStore
Update `crates/storage/src/unified.rs`:
- Add `ttl_index: TTLIndex` field
- Update `put()` with TTL to insert into ttl_index
- Update `delete()` to remove from ttl_index
- Update `find_expired_keys()` to use ttl_index (not full scan)

#### Step 3: Add TTLCleaner Background Task
Create `crates/storage/src/cleaner.rs`:
- `TTLCleaner` struct with background thread
- Periodically calls `find_expired_keys()` and deletes them
- Uses transactions for deletions (proper coordination)

**CRITICAL**: TTL cleanup must use transactions, not direct storage mutation (avoids races).

#### Step 4: Write Tests First (TDD)
Add tests:
1. `test_ttl_index_insert_and_find_expired`
2. `test_ttl_index_remove`
3. `test_find_expired_keys_uses_index` - verify O(expired) not O(total)
4. `test_ttl_cleaner_deletes_expired` - integration test

#### Step 5: Implement to Pass Tests
Implement TTLIndex and TTLCleaner.

#### Step 6: Update lib.rs
```rust
pub mod unified;
pub mod ttl;
pub mod cleaner;

pub use unified::UnifiedStore;
pub use ttl::TTLIndex;
pub use cleaner::TTLCleaner;
```

#### Step 7: Verify
```bash
cargo test -p in-mem-storage
cargo clippy -p in-mem-storage -- -D warnings
cargo fmt -p in-mem-storage
```

### Acceptance Criteria
- [ ] TTLIndex using BTreeMap<Instant, HashSet<Key>>
- [ ] find_expired_keys() uses index (O(expired) not O(total))
- [ ] TTLCleaner background task works
- [ ] Cleanup uses transactions (not direct mutation)
- [ ] All tests pass

### When Complete
```bash
./scripts/complete-story.sh 14
```

### Notes
- **Design**: TTL expiration is LOGICAL delete, not physical
- TTL cleanup runs in background thread
- Cleanup uses transactions to avoid races with active writes

### Repository
https://github.com/anibjoshi/in-mem

---

## Prompt 4: Story #15 - ClonedSnapshotView (WAIT FOR #12)

### Context
You are implementing Story #15 for the in-mem database project. Story #12 (UnifiedStore) is complete. You are now implementing the MVP snapshot mechanism.

### Your Task
Implement ClonedSnapshotView that creates version-bounded views of storage for transactions.

### Prerequisites
1. **WAIT FOR STORY #12 TO MERGE** to `epic-2-storage-layer` branch

2. Once #12 is merged, pull and create your branch:
   ```bash
   git checkout epic-2-storage-layer
   git pull origin epic-2-storage-layer
   git checkout -b epic-2-story-15-snapshot-view
   ```

3. Read context:
   - `docs/architecture/M1_ARCHITECTURE.md` - Snapshot design
   - `crates/core/src/traits.rs` - SnapshotView trait
   - `crates/storage/src/unified.rs` - UnifiedStore implementation
   - GitHub issue #15

### Implementation Steps

#### Step 1: Create snapshot.rs
Create `crates/storage/src/snapshot.rs` with:
- `ClonedSnapshotView` struct
- Fields: `version: u64`, `data: Arc<BTreeMap<Key, VersionedValue>>`
- Implements `SnapshotView` trait

#### Step 2: Implement SnapshotView Methods
Implement trait methods:
- `get()` - lookup in cloned data
- `scan_prefix()` - range scan on cloned data
- `version()` - return snapshot version

#### Step 3: Add create_snapshot to UnifiedStore
Update `crates/storage/src/unified.rs`:
- Add `create_snapshot(&self) -> ClonedSnapshotView` method
- Acquires read lock, clones BTreeMap, returns snapshot
- Snapshot version = current_version()

#### Step 4: Write Tests First (TDD)
Add tests to `snapshot.rs`:
1. `test_snapshot_creation` - snapshot has correct version
2. `test_snapshot_get` - reads from frozen data
3. `test_snapshot_isolation` - writes after snapshot don't appear
4. `test_snapshot_scan_prefix` - range queries work
5. `test_snapshot_is_immutable` - multiple readers don't interfere

#### Step 5: Implement to Pass Tests
Implement ClonedSnapshotView and create_snapshot method.

**Key point**: Snapshot is a deep clone of the BTreeMap at a specific version. Expensive but correct for MVP.

#### Step 6: Update lib.rs
```rust
pub mod unified;
pub mod snapshot;

pub use unified::UnifiedStore;
pub use snapshot::ClonedSnapshotView;
```

#### Step 7: Verify
```bash
cargo test -p in-mem-storage
cargo clippy -p in-mem-storage -- -D warnings
cargo fmt -p in-mem-storage
```

### Acceptance Criteria
- [ ] ClonedSnapshotView implements SnapshotView trait
- [ ] create_snapshot() clones BTreeMap and captures version
- [ ] Snapshots are isolated (writes don't appear)
- [ ] All 5 tests pass
- [ ] No clippy warnings

### When Complete
```bash
./scripts/complete-story.sh 15
```

### Notes
- **Known limitation**: Deep clone is expensive (full BTreeMap copy)
- SnapshotView trait abstraction allows lazy implementation later
- For MVP, correctness > performance

### Repository
https://github.com/anibjoshi/in-mem

---

## Prompt 5: Story #16 - Comprehensive Storage Tests (DO LAST)

### Context
You are implementing Story #16 for the in-mem database project. Stories #12-15 are complete. You are now adding comprehensive integration tests for the storage layer.

### Your Task
Add comprehensive storage integration tests covering all edge cases, concurrent access, and stress scenarios.

### Prerequisites
1. **WAIT FOR STORIES #12, #13, #14, #15 TO MERGE** to `epic-2-storage-layer` branch

2. Once all dependencies merge, pull and create your branch:
   ```bash
   git checkout epic-2-storage-layer
   git pull origin epic-2-storage-layer
   git checkout -b epic-2-story-16-storage-tests
   ```

3. Read context:
   - All prior story implementations (#12-15)
   - `docs/development/TDD_METHODOLOGY.md` - Testing strategy
   - GitHub issue #16

### Implementation Steps

#### Step 1: Create Integration Test File
Create `crates/storage/tests/integration_tests.rs` with comprehensive tests:

1. **Edge Cases**:
   - Empty keys, empty values
   - Very large values (MB-sized)
   - Unicode keys, binary keys
   - Maximum version number (u64::MAX)

2. **Concurrent Access**:
   - 100 threads × 1000 writes
   - Read-heavy workload (90% reads, 10% writes)
   - Write-heavy workload (10% reads, 90% writes)
   - Mixed workload with deletes

3. **TTL and Expiration**:
   - Expired values don't appear in scans
   - find_expired_keys is efficient
   - TTL cleanup doesn't race with writes

4. **Snapshot Isolation**:
   - Snapshots don't see later writes
   - Multiple concurrent snapshots work
   - Large snapshot doesn't crash

5. **Index Consistency**:
   - After 10000 random operations, indices match main storage
   - Scan via index matches scan via full iteration
   - Delete removes from all indices

6. **Version Ordering**:
   - Versions are globally monotonic
   - No version collisions under heavy concurrency
   - current_version() is always accurate

#### Step 2: Add Property-Based Tests (Optional but Recommended)
Use `proptest` or `quickcheck` for property-based testing:
- Random operation sequences maintain consistency
- Invariants hold after arbitrary operations

#### Step 3: Add Stress Tests
Create `crates/storage/tests/stress_tests.rs`:
- Insert 1 million keys
- Scan with 100000 results
- Concurrent snapshot creation under load

#### Step 4: Add Regression Tests
Any bugs found during development should have regression tests.

#### Step 5: Verify Coverage
```bash
# Generate coverage report
cargo tarpaulin -p in-mem-storage --out Html

# Open coverage report
open tarpaulin-report.html

# Ensure ≥85% coverage for storage layer
```

#### Step 6: Run All Tests
```bash
cargo test -p in-mem-storage --all
cargo test -p in-mem-storage --all --release  # Release mode
cargo clippy -p in-mem-storage -- -D warnings
cargo fmt -p in-mem-storage
```

### Acceptance Criteria
- [ ] ≥85% test coverage for storage layer
- [ ] All edge cases tested
- [ ] Concurrent access tests pass (no data races)
- [ ] TTL expiration tests pass
- [ ] Snapshot isolation tests pass
- [ ] Index consistency tests pass
- [ ] Stress tests pass (1M keys, 100K scan results)
- [ ] All tests pass in release mode
- [ ] No clippy warnings

### When Complete
```bash
./scripts/complete-story.sh 16
```

### Notes
- Focus on **correctness** (race conditions, consistency) over performance
- Test **failure scenarios** (expired keys, missing keys, concurrent deletes)
- Use `cargo test --release` to catch optimization bugs

### Repository
https://github.com/anibjoshi/in-mem

---

## Coordination Notes

### For Claude Working on Story #12
- You are **blocking** stories #13, #14, #15
- **Prioritize completion** - get your PR merged ASAP
- Comment on issue #12 when your PR is ready for review
- Ping in issues #13, #14, #15: "Story #12 merged, you can start"

### For Claudes Working on Stories #13, #14, #15
- **Wait for story #12 PR to merge** to epic branch before starting
- You can work in **parallel** with each other (different files)
- If you finish before others, help review their PRs
- All three must merge before #16 can start

### For Claude Working on Story #16
- **Wait for stories #12, #13, #14, #15 to merge** before starting
- Your tests should cover all prior implementations
- Focus on integration tests (multiple components working together)
- This is the quality gate for Epic 2

### Communication Protocol
1. Comment on your GitHub issue when starting work
2. Comment when blocked on dependencies
3. Comment when PR is ready for review
4. Comment when merged (notify downstream dependencies)

---

## Epic 2 Completion

After all 5 stories merge to `epic-2-storage-layer`:
1. Run epic review process:
   ```bash
   ./scripts/review-epic.sh 2
   ```

2. Fill out `docs/milestones/EPIC_2_REVIEW.md`

3. If approved, merge to develop:
   ```bash
   git checkout develop
   git merge epic-2-storage-layer
   git push origin develop
   ```

4. Tag release:
   ```bash
   git tag epic-2-complete
   git push origin epic-2-complete
   ```

5. Close epic issue:
   ```bash
   gh issue close 2
   ```

---

**Epic 2 Repository**: https://github.com/anibjoshi/in-mem
**Epic 2 Branch**: `epic-2-storage-layer`
**Epic 2 Issue**: #2
