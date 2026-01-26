# M13 Implementation Plan: Command Execution Layer (strata-executor)

## Overview

This document provides the high-level implementation plan for M13 (Command Execution Layer).

**Total Scope**: 5 Epics, ~32 Stories

**References**:
- [M13 Executor Specification](./M13_EXECUTOR.md) - Authoritative spec

**Critical Framing**:
> M13 is an **abstraction milestone**, not a feature milestone. It introduces a standardized command interface between all API surfaces and the engine without changing any primitive behavior.
>
> The Command Execution Layer is an in-process execution boundary, not a wire protocol. Commands are typed, serializable operations that represent the complete "instruction set" of Strata.
>
> **M13 does NOT add new capabilities.** It wraps existing primitive operations in a uniform command interface, enabling deterministic replay, thin SDKs, and black-box testing.

**Epic Details**:
- [Epic 90: Command Types](#epic-90-command-types)
- [Epic 91: Output & Error Types](#epic-91-output--error-types)
- [Epic 92: Executor Implementation](#epic-92-executor-implementation)
- [Epic 93: Serialization & JSON Utilities](#epic-93-serialization--json-utilities)
- [Epic 94: Integration & Testing](#epic-94-integration--testing)

---

## Architectural Integration Rules (NON-NEGOTIABLE)

These rules ensure M13 integrates properly with the existing architecture.

### Rule 1: Commands Are Complete

Every public primitive operation MUST have a corresponding Command variant. If an operation cannot be expressed as a command, it is not part of Strata's public behavior.

**FORBIDDEN**: Primitive operations that bypass the command layer, hidden internal-only operations.

### Rule 2: Executor Is Stateless

The Executor dispatches commands to primitives. It holds references to primitives but maintains no state of its own. All state lives in the engine.

**FORBIDDEN**: Caching in the executor, executor-level transactions, executor state that survives restarts.

### Rule 3: Commands Are Self-Contained

Every Command variant contains all information needed to execute. No implicit context, no thread-local state, no ambient configuration.

**FORBIDDEN**: Commands that require external context, implicit run scoping, ambient configuration.

### Rule 4: Output Matches Command

Each Command variant has a deterministic Output type. The same Command on the same state always produces the same Output.

**FORBIDDEN**: Non-deterministic outputs, outputs that vary based on execution context, probabilistic results.

### Rule 5: Errors Are Structured

All errors are represented by the Error enum. No panics, no string-only errors, no error swallowing.

**FORBIDDEN**: Panics in command execution, generic string errors, silent failures.

### Rule 6: Serialization Is Lossless

Commands, Outputs, and Errors MUST serialize and deserialize without loss. Round-trip must be exact.

**FORBIDDEN**: Lossy serialization, type information loss, precision loss.

### Rule 7: Executor Does Not Enforce New Invariants

The Executor dispatches to primitives. Invariant enforcement happens in primitives/engine, not in the executor.

**FORBIDDEN**: Executor-level validation beyond type checking, executor-level invariants.

### Rule 8: No Transport Assumptions

Commands are in-process operations. They do not assume networking, async execution, or remote clients.

**FORBIDDEN**: Network-specific error handling, async requirements, authentication/authorization in commands.

---

## Core Invariants

### Command Invariants (CMD)

| # | Invariant | Test Strategy |
|---|-----------|---------------|
| CMD-1 | Every primitive operation has a Command variant | Exhaustive coverage audit |
| CMD-2 | Commands are self-contained (no external context) | Static analysis, constructor tests |
| CMD-3 | Commands serialize/deserialize losslessly | Round-trip tests for all variants |
| CMD-4 | Command execution is deterministic | Same command + same state = same result |
| CMD-5 | All 48 command variants are typed (no Generic fallback) | Type exhaustiveness tests |

### Output Invariants (OUT)

| # | Invariant | Test Strategy |
|---|-----------|---------------|
| OUT-1 | Output variants cover all return types | Exhaustive coverage audit |
| OUT-2 | Outputs serialize/deserialize losslessly | Round-trip tests for all variants |
| OUT-3 | Output matches expected type for each Command | Type mapping tests |
| OUT-4 | Versioned outputs preserve version metadata | Version preservation tests |

### Error Invariants (ERR)

| # | Invariant | Test Strategy |
|---|-----------|---------------|
| ERR-1 | All primitive errors map to Error variants | Error coverage tests |
| ERR-2 | Errors serialize/deserialize losslessly | Round-trip tests |
| ERR-3 | Errors include structured details | Error detail completeness tests |
| ERR-4 | No error swallowing or transformation | Error propagation tests |

### Executor Invariants (EXE)

| # | Invariant | Test Strategy |
|---|-----------|---------------|
| EXE-1 | Executor is stateless | State isolation tests |
| EXE-2 | execute() dispatches correctly to all 48 variants | Dispatch coverage tests |
| EXE-3 | execute_many() processes sequentially | Order preservation tests |
| EXE-4 | Executor does not modify command semantics | Parity tests vs direct primitive calls |

### Serialization Invariants (SER)

| # | Invariant | Test Strategy |
|---|-----------|---------------|
| SER-1 | JSON encoding handles all Value types | Type coverage tests |
| SER-2 | Special values preserved ($bytes, $f64 wrappers) | Special value round-trip tests |
| SER-3 | Large integers preserved (i64 range) | Numeric precision tests |
| SER-4 | Binary data encoded as base64 | Bytes encoding tests |

---

## Epic Overview

| Epic | Name | Stories | Dependencies | Status |
|------|------|---------|--------------|--------|
| 90 | Command Types | 8 | M11 complete | Pending |
| 91 | Output & Error Types | 6 | Epic 90 | Pending |
| 92 | Executor Implementation | 9 | Epic 90, 91 | Pending |
| 93 | Serialization & JSON Utilities | 5 | Epic 90, 91 | Pending |
| 94 | Integration & Testing | 4 | Epic 92, 93 | Pending |

---

## Epic 90: Command Types

**Goal**: Define the complete Command enum covering all 48 primitive operations

| Story | Description | Priority |
|-------|-------------|----------|
| #700 | Command Enum Structure and RunId Type | FOUNDATION |
| #701 | KV Command Variants (12 variants) | CRITICAL |
| #702 | JSON Command Variants (6 variants) | CRITICAL |
| #703 | Event Command Variants (7 variants) | CRITICAL |
| #704 | State Command Variants (5 variants) | CRITICAL |
| #705 | Vector Command Variants (7 variants) | CRITICAL |
| #706 | Run Command Variants (7 variants) | CRITICAL |
| #707 | Database Command Variants (4 variants) | HIGH |

**Acceptance Criteria**:
- [ ] `Command` enum with exactly 48 variants (no Generic fallback)
- [ ] All commands derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`
- [ ] `RunId` type for run identification (String-based, supports "default" and UUIDs)
- [ ] **KV commands (12)**:
  - `KvGet { run: RunId, key: String }`
  - `KvPut { run: RunId, key: String, value: Value }`
  - `KvDelete { run: RunId, key: String }`
  - `KvExists { run: RunId, key: String }`
  - `KvGetAt { run: RunId, key: String, version: u64 }`
  - `KvHistory { run: RunId, key: String, limit: Option<u64> }`
  - `KvScan { run: RunId, prefix: String, limit: Option<u64>, cursor: Option<String> }`
  - `KvMget { run: RunId, keys: Vec<String> }`
  - `KvMput { run: RunId, entries: Vec<(String, Value)> }`
  - `KvMdelete { run: RunId, keys: Vec<String> }`
  - `KvIncr { run: RunId, key: String, delta: i64 }`
  - `KvCas { run: RunId, key: String, expected: Option<Value>, new_value: Value }`
- [ ] **JSON commands (6)**:
  - `JsonGet { run: RunId, key: String }`
  - `JsonSet { run: RunId, key: String, value: Value }`
  - `JsonGetPath { run: RunId, key: String, path: String }`
  - `JsonSetPath { run: RunId, key: String, path: String, value: Value }`
  - `JsonDeletePath { run: RunId, key: String, path: String }`
  - `JsonMergePatch { run: RunId, key: String, patch: Value }`
- [ ] **Event commands (7)**:
  - `EventAppend { run: RunId, stream: String, event_type: String, payload: Value }`
  - `EventRead { run: RunId, stream: String, start: u64, limit: u64 }`
  - `EventReadRange { run: RunId, stream: String, start: u64, end: u64 }`
  - `EventReadByType { run: RunId, stream: String, event_type: String }`
  - `EventLatest { run: RunId, stream: String }`
  - `EventCount { run: RunId, stream: String }`
  - `EventVerifyChain { run: RunId, stream: String }`
- [ ] **State commands (5)**:
  - `StateGet { run: RunId, cell: String }`
  - `StateSet { run: RunId, cell: String, value: Value }`
  - `StateTransition { run: RunId, cell: String, from: Value, to: Value }`
  - `StateDelete { run: RunId, cell: String }`
  - `StateList { run: RunId }`
- [ ] **Vector commands (7)**:
  - `VectorCreateCollection { run: RunId, name: String, dimensions: usize, metric: DistanceMetric }`
  - `VectorDeleteCollection { run: RunId, name: String }`
  - `VectorInsert { run: RunId, collection: String, id: String, embedding: Vec<f32>, metadata: Option<Value> }`
  - `VectorSearch { run: RunId, collection: String, query: Vec<f32>, k: usize, filter: Option<MetadataFilter> }`
  - `VectorGet { run: RunId, collection: String, id: String }`
  - `VectorDelete { run: RunId, collection: String, id: String }`
  - `VectorCount { run: RunId, collection: String }`
- [ ] **Run commands (7)**:
  - `RunCreate { name: Option<String>, metadata: Option<Value> }`
  - `RunGet { run: RunId }`
  - `RunList { status: Option<RunStatus>, limit: Option<u64> }`
  - `RunUpdateStatus { run: RunId, status: RunStatus }`
  - `RunUpdateMetadata { run: RunId, metadata: Value }`
  - `RunDelete { run: RunId }`
  - `RunExport { run: RunId, path: String }`
- [ ] **Database commands (4)**:
  - `Ping`
  - `Info`
  - `Flush`
  - `Compact`
- [ ] All field types use established core types (Value, RunId, etc.)
- [ ] Unit tests for construction of all 48 variants

---

## Epic 91: Output & Error Types

**Goal**: Define Output enum for successful results and Error enum for failures

| Story | Description | Priority |
|-------|-------------|----------|
| #710 | Output Enum Core Variants | FOUNDATION |
| #711 | Output Versioned and Collection Variants | CRITICAL |
| #712 | VersionedValue and Supporting Types | CRITICAL |
| #713 | Error Enum Implementation | CRITICAL |
| #714 | Error Detail Types | HIGH |
| #715 | Command-Output Type Mapping Documentation | HIGH |

**Acceptance Criteria**:
- [ ] `Output` enum with all return type variants:
  - `Unit` - No return value (delete, flush)
  - `Value(Value)` - Single value
  - `Versioned { value: Value, version: u64, timestamp: u64 }` - Value with version
  - `Maybe(Option<Value>)` - Optional value (get operations)
  - `MaybeVersioned(Option<VersionedValue>)` - Optional versioned value
  - `Values(Vec<Option<Value>>)` - Multiple optional values (mget)
  - `Version(u64)` - Version number only
  - `Bool(bool)` - Boolean result
  - `Int(i64)` - Integer result (count, incr)
  - `Keys(Vec<String>)` - List of keys
  - `Events(Vec<Event>)` - List of events
  - `History(Vec<VersionedValue>)` - Version history
  - `SearchResults(Vec<SearchResult>)` - Vector search results
  - `Run(RunInfo)` - Single run info
  - `Runs(Vec<RunInfo>)` - Multiple run infos
  - `Info(DatabaseInfo)` - Database info
  - `Pong { version: String }` - Ping response
  - `Scan { keys: Vec<String>, cursor: Option<String> }` - Scan results
  - `ChainValid(bool)` - Chain verification result
- [ ] `VersionedValue` struct: `{ value: Value, version: u64, timestamp: u64 }`
- [ ] `Event` struct for event log entries
- [ ] `SearchResult` struct for vector search results
- [ ] `RunInfo` struct for run metadata
- [ ] `DatabaseInfo` struct for database info
- [ ] All Output variants derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`
- [ ] `Error` enum with all error cases:
  - `KeyNotFound { key: String }`
  - `RunNotFound { run: String }`
  - `CollectionNotFound { collection: String }`
  - `StreamNotFound { stream: String }`
  - `WrongType { expected: String, actual: String }`
  - `InvalidKey { reason: String }`
  - `InvalidPath { reason: String }`
  - `VersionConflict { expected: u64, actual: u64 }`
  - `TransitionFailed { expected: String, actual: String }`
  - `RunClosed { run: String }`
  - `RunExists { run: String }`
  - `CollectionExists { collection: String }`
  - `DimensionMismatch { expected: usize, actual: usize }`
  - `ConstraintViolation { reason: String }`
  - `HistoryTrimmed { requested: u64, earliest: u64 }`
  - `Overflow { reason: String }`
  - `Io { reason: String }`
  - `Serialization { reason: String }`
  - `Internal { reason: String }`
- [ ] Error implements `std::error::Error` and `Display`
- [ ] Error derives `Serialize`, `Deserialize`, `Clone`, `Debug`
- [ ] **Command-Output type mapping documented**:

| Command Category | Output Type |
|------------------|-------------|
| KvGet | `MaybeVersioned` |
| KvPut | `Version` |
| KvDelete | `Bool` |
| KvExists | `Bool` |
| KvGetAt | `Versioned` or `HistoryTrimmed` error |
| KvHistory | `History` |
| KvScan | `Scan` |
| KvMget | `Values` |
| KvMput | `Unit` |
| KvMdelete | `Int` (count deleted) |
| KvIncr | `Int` |
| KvCas | `Bool` |
| JsonGet | `MaybeVersioned` |
| JsonSet | `Version` |
| JsonGetPath | `Maybe` |
| JsonSetPath | `Version` |
| JsonDeletePath | `Bool` |
| JsonMergePatch | `Version` |
| EventAppend | `Version` |
| EventRead | `Events` |
| EventReadRange | `Events` |
| EventReadByType | `Events` |
| EventLatest | `MaybeVersioned` |
| EventCount | `Int` |
| EventVerifyChain | `ChainValid` |
| StateGet | `MaybeVersioned` |
| StateSet | `Version` |
| StateTransition | `Bool` |
| StateDelete | `Bool` |
| StateList | `Keys` |
| VectorCreateCollection | `Unit` |
| VectorDeleteCollection | `Bool` |
| VectorInsert | `Version` |
| VectorSearch | `SearchResults` |
| VectorGet | `MaybeVersioned` |
| VectorDelete | `Bool` |
| VectorCount | `Int` |
| RunCreate | `Run` |
| RunGet | `Run` or `RunNotFound` error |
| RunList | `Runs` |
| RunUpdateStatus | `Unit` |
| RunUpdateMetadata | `Version` |
| RunDelete | `Unit` |
| RunExport | `Unit` |
| Ping | `Pong` |
| Info | `Info` |
| Flush | `Unit` |
| Compact | `Unit` |

---

## Epic 92: Executor Implementation

**Goal**: Implement the Executor that dispatches commands to primitives

| Story | Description | Priority |
|-------|-------------|----------|
| #720 | Executor Struct and Constructor | FOUNDATION |
| #721 | KV Command Handlers (12 handlers) | CRITICAL |
| #722 | JSON Command Handlers (6 handlers) | CRITICAL |
| #723 | Event Command Handlers (7 handlers) | CRITICAL |
| #724 | State Command Handlers (5 handlers) | CRITICAL |
| #725 | Vector Command Handlers (7 handlers) | CRITICAL |
| #726 | Run Command Handlers (7 handlers) | CRITICAL |
| #727 | Database Command Handlers (4 handlers) | HIGH |
| #728 | Error Conversion Layer | CRITICAL |

**Acceptance Criteria**:
- [ ] `Executor` struct holding references to all primitives:
  ```rust
  pub struct Executor {
      engine: Arc<Database>,
      // Primitive handles derived from engine
  }
  ```
- [ ] `Executor::new(engine: Arc<Database>) -> Self`
- [ ] `Executor::execute(&self, cmd: Command) -> Result<Output, Error>`
- [ ] `Executor::execute_many(&self, cmds: Vec<Command>) -> Vec<Result<Output, Error>>`
- [ ] Match dispatch covering all 48 command variants
- [ ] **KV handlers**:
  - `kv_get` → calls `kv.get()`, converts to `MaybeVersioned`
  - `kv_put` → calls `kv.put()`, returns `Version`
  - `kv_delete` → calls `kv.delete()`, returns `Bool`
  - `kv_exists` → calls `kv.exists()`, returns `Bool`
  - `kv_get_at` → calls `kv.get_at()`, returns `Versioned` or error
  - `kv_history` → calls `kv.history()`, returns `History`
  - `kv_scan` → calls `kv.scan()`, returns `Scan`
  - `kv_mget` → calls `kv.mget()`, returns `Values`
  - `kv_mput` → calls `kv.mput()`, returns `Unit`
  - `kv_mdelete` → calls `kv.mdelete()`, returns `Int`
  - `kv_incr` → calls `kv.incr()`, returns `Int`
  - `kv_cas` → calls `kv.cas()`, returns `Bool`
- [ ] **JSON handlers**: All 6 operations mapped correctly
- [ ] **Event handlers**: All 7 operations mapped correctly
- [ ] **State handlers**: All 5 operations mapped correctly
- [ ] **Vector handlers**: All 7 operations mapped correctly
- [ ] **Run handlers**: All 7 operations mapped correctly
- [ ] **Database handlers**:
  - `Ping` → returns `Pong { version: env!("CARGO_PKG_VERSION") }`
  - `Info` → returns `Info(DatabaseInfo)`
  - `Flush` → calls `engine.flush()`, returns `Unit`
  - `Compact` → calls `engine.compact()`, returns `Unit`
- [ ] **Error conversion**:
  - Internal `strata_core::Error` maps to `executor::Error`
  - No error information lost
  - Structured details preserved
- [ ] All handlers are synchronous (no async)
- [ ] Executor is `Send + Sync`
- [ ] Unit tests for each handler

---

## Epic 93: Serialization & JSON Utilities

**Goal**: Implement JSON serialization for CLI and MCP output formatting

| Story | Description | Priority |
|-------|-------------|----------|
| #730 | Value JSON Encoding | CRITICAL |
| #731 | Special Value Wrappers ($bytes, $f64) | CRITICAL |
| #732 | Output JSON Encoding | HIGH |
| #733 | Error JSON Encoding | HIGH |
| #734 | Command JSON Encoding | HIGH |

**Acceptance Criteria**:
- [ ] `Value` → JSON encoding:
  - `Null` → `null`
  - `Bool(b)` → `true`/`false`
  - `Int(n)` → number (as JSON number if in safe range)
  - `Float(f)` → number (normal) or `{"$f64": "NaN|+Inf|-Inf|-0.0"}` (special)
  - `String(s)` → `"string"`
  - `Bytes(b)` → `{"$bytes": "<base64>"}`
  - `Array(a)` → `[...]`
  - `Object(o)` → `{...}`
- [ ] Special float handling:
  - `NaN` → `{"$f64": "NaN"}`
  - `+Infinity` → `{"$f64": "+Inf"}`
  - `-Infinity` → `{"$f64": "-Inf"}`
  - `-0.0` → `{"$f64": "-0.0"}`
- [ ] Bytes encoding uses standard base64 (RFC 4648)
- [ ] JSON → `Value` decoding:
  - Recognizes `$bytes` wrapper
  - Recognizes `$f64` wrapper
  - Numbers decode to `Int` if no decimal, `Float` if decimal
  - Large integers (> i64::MAX) handled gracefully (error or BigInt representation)
- [ ] `Output` JSON encoding for all variants
- [ ] `Error` JSON encoding: `{"code": "...", "message": "...", "details": {...}}`
- [ ] `Command` JSON encoding for debugging/logging
- [ ] Round-trip tests for all types
- [ ] No precision loss for i64 integers
- [ ] No precision loss for f64 floats (via wrappers)

---

## Epic 94: Integration & Testing

**Goal**: Integrate executor with existing API layer and comprehensive testing

| Story | Description | Priority |
|-------|-------------|----------|
| #740 | Workspace Integration | CRITICAL |
| #741 | Executor-Primitive Parity Tests | CRITICAL |
| #742 | Serialization Round-Trip Tests | CRITICAL |
| #743 | Determinism Verification Tests | HIGH |

**Acceptance Criteria**:
- [ ] `strata-executor` crate added to workspace
- [ ] Crate dependencies:
  ```toml
  [dependencies]
  strata-core = { path = "../core" }
  strata-engine = { path = "../engine" }
  strata-primitives = { path = "../primitives" }
  serde = { workspace = true }
  serde_json = { workspace = true }
  thiserror = { workspace = true }
  base64 = { workspace = true }
  ```
- [ ] Public API exports:
  - `Command` enum
  - `Output` enum
  - `Error` enum
  - `Executor` struct
  - `VersionedValue`, `Event`, `SearchResult`, `RunInfo`, `DatabaseInfo`
- [ ] **Parity tests**: Every command produces same result as direct primitive call
- [ ] **Round-trip tests**: All commands, outputs, errors survive JSON round-trip
- [ ] **Determinism tests**: Same command sequence on same initial state = same results
- [ ] **Coverage**: All 48 command variants have execution tests
- [ ] **Error coverage**: All error variants have trigger tests
- [ ] Integration test: Full workflow (create run → operations → export)
- [ ] Benchmark: Command dispatch overhead < 100ns

---

## Files to Create/Modify

### New Files

| File | Description |
|------|-------------|
| `crates/executor/Cargo.toml` | Crate manifest |
| `crates/executor/src/lib.rs` | Public API, re-exports |
| `crates/executor/src/command.rs` | Command enum (48 variants) |
| `crates/executor/src/output.rs` | Output enum and supporting types |
| `crates/executor/src/error.rs` | Error enum |
| `crates/executor/src/executor.rs` | Executor implementation |
| `crates/executor/src/handlers/mod.rs` | Handler module |
| `crates/executor/src/handlers/kv.rs` | KV command handlers |
| `crates/executor/src/handlers/json.rs` | JSON command handlers |
| `crates/executor/src/handlers/event.rs` | Event command handlers |
| `crates/executor/src/handlers/state.rs` | State command handlers |
| `crates/executor/src/handlers/vector.rs` | Vector command handlers |
| `crates/executor/src/handlers/run.rs` | Run command handlers |
| `crates/executor/src/handlers/database.rs` | Database command handlers |
| `crates/executor/src/convert.rs` | Error conversion from internal errors |
| `crates/executor/src/json.rs` | JSON encoding utilities |
| `crates/executor/src/types.rs` | Supporting types (VersionedValue, etc.) |

### Test Files

| File | Description |
|------|-------------|
| `crates/executor/tests/command_tests.rs` | Command construction and serialization |
| `crates/executor/tests/output_tests.rs` | Output type tests |
| `crates/executor/tests/error_tests.rs` | Error type tests |
| `crates/executor/tests/executor_tests.rs` | Executor dispatch tests |
| `crates/executor/tests/parity_tests.rs` | Command vs primitive parity |
| `crates/executor/tests/roundtrip_tests.rs` | JSON round-trip tests |
| `crates/executor/tests/determinism_tests.rs` | Determinism verification |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `crates/executor` to workspace members |
| `crates/api/src/lib.rs` | Optional: expose executor for API consumers |

---

## Dependency Order

```
Epic 90 (Command Types)
    ↓
Epic 91 (Output & Error Types) ←── Epic 90
    ↓
    ├───────────────┐
    ↓               ↓
Epic 92         Epic 93
(Executor)      (Serialization)
    ↓               ↓
    └───────┬───────┘
            ↓
Epic 94 (Integration & Testing)
            ↓
════════════════════════════════
        M13 COMPLETE
════════════════════════════════
```

**Recommended Implementation Order**:
1. Epic 90: Command Types (foundation - defines all operations)
2. Epic 91: Output & Error Types (defines all results)
3. Epic 93: Serialization & JSON Utilities (can be done in parallel with 92)
4. Epic 92: Executor Implementation (brings it all together)
5. Epic 94: Integration & Testing (validates everything)

---

## Phased Implementation Strategy

> **Guiding Principle**: Define types first. Commands must be complete before the executor. Serialization should work independently. Integration validates the full stack.

### Phase 1: Type Foundation

Define all core types without implementation:
- Command enum with all 48 variants
- Output enum with all variants
- Error enum with all variants
- Supporting types (VersionedValue, Event, etc.)

**Exit Criteria**: All types compile. All types serialize/deserialize. No implementation yet.

### Phase 2: Serialization

Implement JSON encoding/decoding:
- Value JSON encoding with special wrappers
- Output JSON encoding
- Error JSON encoding
- Round-trip tests passing

**Exit Criteria**: All types survive JSON round-trip. Special values handled correctly.

### Phase 3: Executor Core

Implement the executor:
- Executor struct and constructor
- Match dispatch for all 48 commands
- Error conversion layer
- All handlers implemented

**Exit Criteria**: All commands execute correctly. Parity with direct primitive calls.

### Phase 4: Integration (M13 Exit Gate)

Final integration and validation:
- Workspace integration
- Comprehensive test coverage
- Determinism verification
- Performance validation

**Exit Criteria**: All tests pass. Ready for API layer integration.

### Phase Summary

| Phase | Epics | Key Deliverable | Status |
|-------|-------|-----------------|--------|
| 1 | 90, 91 | Type definitions | Pending |
| 2 | 93 | JSON serialization | Pending |
| 3 | 92 | Executor implementation | Pending |
| 4 | 94 | Integration & testing | Pending |

---

## Testing Strategy

### Unit Tests

- Command variant construction (all 48)
- Command field validation
- Output variant construction (all 19)
- Error variant construction (all 19)
- JSON encoding for each Value type
- Special wrapper encoding ($bytes, $f64)
- Error conversion from internal errors

### Integration Tests

- Full command execution flow
- Multi-command sequences
- Run-scoped operations
- Cross-primitive workflows
- Error propagation through executor

### Parity Tests

- Every command vs direct primitive call
- Same inputs produce same outputs
- Same errors for same invalid inputs
- Version numbers match
- Timestamps in expected range

### Round-Trip Tests

- All 48 command variants through JSON
- All 19 output variants through JSON
- All 19 error variants through JSON
- Special float values (NaN, Inf, -0.0)
- Binary data (bytes)
- Large integers (i64 boundaries)
- Unicode strings
- Nested objects and arrays

### Determinism Tests

- Same command sequence produces same state
- Order matters (verify ordering)
- Repeated execution produces same results
- No time-dependent behavior in commands

### Performance Tests

- Command dispatch overhead (target: <100ns)
- Serialization overhead (target: <1μs per command)
- No allocation in hot path
- Memory usage stable

---

## Success Metrics

**Functional**: All ~32 stories passing, 100% acceptance criteria met

**Type Coverage**:
- All 48 command variants implemented
- All 19 output variants implemented
- All 19 error variants implemented
- No Generic/Any fallbacks

**Serialization**:
- 100% round-trip accuracy
- All special values preserved
- No precision loss

**Parity**:
- Every command produces identical results to direct primitive calls
- Error behavior matches primitive error behavior

**Performance**:
- Dispatch overhead < 100ns
- No measurable impact on primitive performance

**Quality**: Test coverage > 95% for executor crate

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing command variants | Low | High | Systematic audit against primitives |
| Type mismatch in Output | Medium | Medium | Comprehensive type mapping tests |
| Serialization precision loss | Medium | High | Extensive round-trip tests, special wrappers |
| Performance overhead | Low | Medium | Benchmark early, optimize dispatch |
| Error information loss | Medium | Medium | Error conversion tests, detail preservation |
| Breaking existing code | Low | High | Executor is additive, old API unchanged |

---

## Not In Scope (Explicitly Deferred)

1. **Async execution** - Post-MVP (Commands are synchronous)
2. **Transaction batching** - Post-MVP (`execute_atomic` is placeholder)
3. **Command middleware** - Post-MVP (logging, metrics)
4. **Command replay infrastructure** - Post-MVP (logging, replay CLI)
5. **Remote execution** - Post-MVP (server integration)
6. **Command versioning** - Post-MVP (for wire protocol evolution)
7. **Batch optimization** - Post-MVP (mget/mput optimization)
8. **Streaming results** - Post-MVP (large result sets)

---

## Post-M13 Expectations

After M13 completion:
1. Every Strata operation expressible as a typed Command
2. Commands are self-contained and serializable
3. Executor provides single entry point to all primitives
4. JSON encoding handles all edge cases (special floats, bytes)
5. Black-box testing enabled (feed commands, assert results)
6. Deterministic replay possible (same commands = same state)
7. Foundation ready for thin SDKs (Python, Node, CLI)
8. No performance regression from command abstraction
9. Wire protocol (future M14) has clean command interface to build on
10. RunBundle integration straightforward (commands as semantic log)

---

## Command Count Summary

| Primitive | Commands | Variants |
|-----------|----------|----------|
| KV | 12 | Get, Put, Delete, Exists, GetAt, History, Scan, Mget, Mput, Mdelete, Incr, Cas |
| JSON | 6 | Get, Set, GetPath, SetPath, DeletePath, MergePatch |
| Events | 7 | Append, Read, ReadRange, ReadByType, Latest, Count, VerifyChain |
| State | 5 | Get, Set, Transition, Delete, List |
| Vectors | 7 | CreateCollection, DeleteCollection, Insert, Search, Get, Delete, Count |
| Runs | 7 | Create, Get, List, UpdateStatus, UpdateMetadata, Delete, Export |
| Database | 4 | Ping, Info, Flush, Compact |
| **Total** | **48** | |

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-25 | Initial M13 implementation plan |

---

**This is the implementation plan. All work must conform to it.**
