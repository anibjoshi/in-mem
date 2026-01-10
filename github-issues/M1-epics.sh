#!/bin/bash
# M1 Epic Creation Script

REPO="anibjoshi/in-mem"
GH="/opt/homebrew/bin/gh"

# Create labels first (ignore errors if they exist)
echo "Creating labels..."
$GH label create "milestone-1" --color "0E8A16" --description "Milestone 1: Foundation" --repo $REPO 2>/dev/null
$GH label create "epic" --color "5319E7" --description "Epic - large feature area" --repo $REPO 2>/dev/null
$GH label create "priority-high" --color "D93F0B" --description "High priority" --repo $REPO 2>/dev/null
$GH label create "priority-medium" --color "FBCA04" --description "Medium priority" --repo $REPO 2>/dev/null
$GH label create "risk-high" --color "B60205" --description "High risk" --repo $REPO 2>/dev/null
$GH label create "risk-medium" --color "FF9800" --description "Medium risk" --repo $REPO 2>/dev/null

echo "Creating M1 Epics..."

# Epic 1: Workspace & Core Types
$GH issue create \
  --repo $REPO \
  --title "[M1 Epic] Workspace & Core Types" \
  --label "milestone-1,epic,priority-high" \
  --body "## Epic: Workspace & Core Types

**Goal**: Set up Cargo workspace and define core type system that all other components depend on.

### Scope
- Cargo workspace configuration
- Core type definitions (RunId, Namespace, Key, TypeTag, Value)
- Error type hierarchy
- Trait definitions (Storage, SnapshotView)

### Success Criteria
- [ ] Workspace builds with all crates
- [ ] Core types are well-documented
- [ ] Type system compiles without warnings
- [ ] Traits define clear contracts

### Dependencies
- None (this is the foundation)

### Estimated Effort
3-4 days

### Risks
- **Risk**: Poor type design will require refactoring later
- **Mitigation**: Review with architecture doc, keep types simple for MVP

### User Stories
Will be broken down into separate issues:
- Setup Cargo workspace
- Define RunId and Namespace types
- Define Key and TypeTag enums
- Define Value enum and VersionedValue
- Define Error types
- Define Storage and SnapshotView traits
"

# Epic 2: Storage Layer
$GH issue create \
  --repo $REPO \
  --title "[M1 Epic] Storage Layer" \
  --label "milestone-1,epic,priority-high,risk-medium" \
  --body "## Epic: Storage Layer

**Goal**: Implement in-memory storage with BTreeMap backend, versioning, and indexing.

### Scope
- UnifiedStore struct with BTreeMap
- Version management (AtomicU64)
- Secondary indices (run_index, type_index)
- TTL index structure
- Storage trait implementation

### Success Criteria
- [ ] Can store and retrieve VersionedValue by Key
- [ ] Version numbers are monotonically increasing
- [ ] Secondary indices are maintained correctly
- [ ] Storage trait fully implemented
- [ ] Unit tests cover all operations

### Dependencies
- Epic: Workspace & Core Types (must complete first)

### Estimated Effort
4-5 days

### Risks
- **Risk**: RwLock contention may show up early in testing
- **Mitigation**: Accept for MVP, Storage trait allows future replacement
- **Risk**: Index maintenance bugs
- **Mitigation**: Comprehensive unit tests

### User Stories
Will be broken down into separate issues:
- Implement UnifiedStore with BTreeMap
- Add version management
- Implement secondary indices
- Add TTL index structure
- Implement Storage trait
- Add comprehensive unit tests
"

# Epic 3: WAL Implementation
$GH issue create \
  --repo $REPO \
  --title "[M1 Epic] WAL Implementation" \
  --label "milestone-1,epic,priority-high,risk-high" \
  --body "## Epic: WAL Implementation

**Goal**: Implement append-only Write-Ahead Log for durability.

### Scope
- WAL entry types (BeginTxn, Write, Delete, CommitTxn, etc.)
- Entry encoding/decoding (bincode or similar)
- File I/O with fsync
- WAL append and read operations
- Configurable durability modes (strict/batched/async)

### Success Criteria
- [ ] WAL entries can be appended and read back
- [ ] All entry types serialize/deserialize correctly
- [ ] File I/O is correct (no corruption)
- [ ] Durability modes work as expected
- [ ] Unit tests cover encoding and I/O

### Dependencies
- Epic: Core Types (for WALEntry definitions)

### Estimated Effort
4-5 days

### Risks
- **Risk**: Data corruption bugs can cause data loss
- **Mitigation**: Extensive testing, CRC checks, corruption simulation tests
- **Risk**: fsync performance issues
- **Mitigation**: Default to batched mode, make configurable

### User Stories
Will be broken down into separate issues:
- Define WAL entry types
- Implement entry encoding/decoding
- Implement WAL file operations
- Add fsync with durability modes
- Add CRC/checksums for corruption detection
- Write corruption simulation tests
"

# Epic 4: Basic Recovery
$GH issue create \
  --repo $REPO \
  --title "[M1 Epic] Basic Recovery" \
  --label "milestone-1,epic,priority-high,risk-high" \
  --body "## Epic: Basic Recovery

**Goal**: Implement startup recovery by replaying WAL entries.

### Scope
- Recovery flow: scan WAL, replay committed transactions
- Discard incomplete transactions (no CommitTxn)
- Restore UnifiedStore state from WAL
- Integration with Database startup
- Basic crash simulation tests

### Success Criteria
- [ ] Can recover from clean shutdown (all transactions committed)
- [ ] Discards incomplete transactions correctly
- [ ] Restored state matches pre-crash state
- [ ] Crash simulation tests pass
- [ ] Recovery completes in reasonable time

### Dependencies
- Epic: Storage Layer (need UnifiedStore)
- Epic: WAL Implementation (need WAL reading)

### Estimated Effort
3-4 days

### Risks
- **Risk**: Recovery bugs cause data loss or corruption
- **Mitigation**: Extensive crash simulation tests, property-based testing
- **Risk**: Slow recovery with large WAL
- **Mitigation**: Accept for MVP, snapshots in M4 will fix

### User Stories
Will be broken down into separate issues:
- Implement WAL replay logic
- Handle incomplete transactions
- Integrate recovery with Database::open()
- Add crash simulation tests
- Test recovery with large WAL files
"

# Epic 5: Database Engine Shell
$GH issue create \
  --repo $REPO \
  --title "[M1 Epic] Database Engine Shell" \
  --label "milestone-1,epic,priority-medium" \
  --body "## Epic: Database Engine Shell

**Goal**: Create basic Database struct that orchestrates storage and WAL (transactions come in M2).

### Scope
- Database struct with UnifiedStore and WAL
- Database::open() with recovery
- Basic run tracking (begin_run, end_run)
- Simple put/get operations (no transactions yet)
- KV primitive facade (basic version)

### Success Criteria
- [ ] Database::open() succeeds with recovery
- [ ] Can begin/end runs
- [ ] Basic put/get works
- [ ] WAL is appended on writes
- [ ] Integration test: write, restart, read

### Dependencies
- Epic: Storage Layer
- Epic: WAL Implementation
- Epic: Basic Recovery

### Estimated Effort
3-4 days

### Risks
- **Risk**: Over-engineering the engine before transactions are ready
- **Mitigation**: Keep it minimal, just enough to integrate storage + WAL

### User Stories
Will be broken down into separate issues:
- Create Database struct
- Implement Database::open() with recovery
- Add run tracking (begin_run, end_run)
- Implement basic put/get (non-transactional)
- Write integration test (write, restart, read)
"

echo ""
echo "✅ Created 5 M1 Epics!"
echo ""
echo "Next steps:"
echo "1. View issues: $GH issue list --repo $REPO --label milestone-1"
echo "2. Create user stories for each epic"
echo "3. Assign issues to milestone"
