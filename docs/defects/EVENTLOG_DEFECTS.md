# EventLog Defects and Gaps

> Consolidated from test suite analysis and architecture review.
> Source: `tests/substrate_api_comprehensive/eventlog/` and `crates/api/src/substrate/event.rs`

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| Implementation Bugs | 4 | P0-P1 |
| Known Limitations | 1 | N/A |
| **Total Bugs** | **4** | |

---

## P0: Implementation Bugs (4)

### 1. Payload Type Validation Not Enforced

**Tests:**
- `eventlog::edge_cases::test_payload_must_be_object_null_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_bool_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_int_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_float_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_string_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_bytes_rejected`
- `eventlog::edge_cases::test_payload_must_be_object_array_rejected`

**Expected:** `event_append(&run, stream, non_object_value)` should return `ConstraintViolation` error

**Actual:** Operation succeeds with any Value type

**Contract Reference:** From `crates/api/src/substrate/event.rs` lines 30-43:
> "Payload not Object" → `ConstraintViolation`

**Fix Location:** Add payload type check in `SubstrateImpl::event_append()` or desugar layer

---

### 2. Empty Stream Name Not Rejected

**Test:** `eventlog::edge_cases::test_stream_name_empty_rejected`

**Expected:** `event_append(&run, "", payload)` should return `InvalidKey` error

**Actual:** Operation succeeds

**Contract Reference:** Section 11.3 - "Invalid stream name" → `InvalidKey`

**Fix Location:** Stream name validation in `SubstrateImpl::event_append()` or desugar layer

---

### 3. Sequence Numbers Start at 0 Instead of 1

**Test:** `eventlog::basic_ops::test_append_returns_sequence_version`

**Expected:** First event appended should have sequence >= 1

**Actual:** First event has sequence 0

**Analysis:** This may be intentional (0-based indexing) but conflicts with the expectation that sequence numbers are positive. The test asserts `n >= 1` per typical database conventions.

**Fix Options:**
1. Change primitive to use 1-based sequences (breaking change)
2. Update documentation and tests to use 0-based sequences (documentation fix)

---

### 4. Float Special Values (NaN, Infinity) Not Roundtrip-Safe

**Test:** `eventlog::edge_cases::test_float_special_values_in_payload`

**Expected:** Float values including `f64::NAN`, `f64::INFINITY`, `f64::NEG_INFINITY` should either:
- Be rejected at append time with clear error, OR
- Be stored and retrieved correctly

**Actual:** Append succeeds, but reading back fails with serialization error:
```
Serialization { message: "invalid type: null, expected f64 at line 1 column 108" }
```

**Root Cause:** JSON serialization doesn't support NaN/Infinity natively. The values are serialized as `null` but deserialization expects `f64`.

**Fix Options:**
1. Validate payloads for special float values at append time
2. Use a JSON extension format that supports special floats
3. Document as known limitation (JSON constraint)

---

## Known Limitations (Not Bugs)

### L1. Sequences Are Global, Not Per-Stream

**Tests:** `eventlog::streams::test_sequences_are_global_not_per_stream`

**Behavior:** Event sequences are global across all streams within a run, not per-stream like Redis Streams.

**Example:**
```rust
event_append(&run, "stream1", payload1);  // seq = 0
event_append(&run, "stream2", payload2);  // seq = 1  (not 0!)
event_append(&run, "stream1", payload3);  // seq = 2  (not 1!)
```

**Status:** Documented in M11 contract; accepted limitation for M11.

**Root Cause:** Substrate maps `stream` → `event_type` field in primitive. Primitive has single log per run with global sequence space.

---

## Performance Notes

### event_len() and event_latest_sequence() are O(n)

**From architecture analysis:**
- `event_len(&run, stream)` reads all events and filters by type
- `event_latest_sequence(&run, stream)` reverse scans until type match

**Recommendation:** Add primitive-level support for `len_by_type()` and `latest_by_type()`

---

## Priority Matrix

| ID | Issue | Priority | Effort | Dependencies |
|----|-------|----------|--------|--------------|
| 1 | Payload type validation | P0 | Low | None |
| 2 | Empty stream name validation | P1 | Low | None |
| 3 | Sequence numbering (0 vs 1) | P1 | Low | Documentation or Code |
| 4 | Float special values | P2 | Medium | JSON serialization |

---

## Recommended Fix Order

### Phase 1: Quick Wins (P0-P1, Low Effort)
1. Add payload type validation (must be Object)
2. Add stream name validation (must be non-empty)
3. Clarify sequence numbering convention (document or fix)

### Phase 2: Edge Cases (P2, Medium Effort)
4. Handle float special values (validate or support)

### Phase 3: Performance (P2, Medium Effort)
5. Add primitive support for type-filtered count and latest operations

---

## Test Coverage Summary

| Current | After Phase 1 | After Phase 2 |
|---------|---------------|---------------|
| 82 pass | 91 pass | 92 pass |
| 10 fail | 1 fail | 0 fail |
| 0 ignore | 0 ignore | 0 ignore |

---

## EventLog Test Suite Summary

| Module | Tests | Pass | Fail | Notes |
|--------|-------|------|------|-------|
| basic_ops | 22 | 21 | 1 | Sequence numbering |
| streams | 10 | 10 | 0 | Multi-stream behavior |
| edge_cases | 27 | 18 | 9 | Validation failures |
| durability | 12 | 12 | 0 | Crash recovery |
| concurrency | 9 | 9 | 0 | Thread safety |
| recovery_invariants | 12 | 12 | 0 | Recovery guarantees |
| **Total** | **92** | **82** | **10** | |

---

## GitHub Issues

| Issue | Title | Priority |
|-------|-------|----------|
| [#705](https://github.com/anibjoshi/in-mem/issues/705) | Payload type validation not enforced | P0 |
| [#706](https://github.com/anibjoshi/in-mem/issues/706) | Empty stream name not rejected | P1 |
| [#707](https://github.com/anibjoshi/in-mem/issues/707) | Sequences start at 0 instead of 1 | P1 |
| [#708](https://github.com/anibjoshi/in-mem/issues/708) | Float NaN/Infinity values fail on read | P2 |
