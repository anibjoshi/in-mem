//! Audit test for issue #920: Input validation applied at inconsistent layers
//! Verdict: ARCHITECTURAL CHOICE
//!
//! This issue is purely structural/architectural. Input validation is applied at
//! different layers depending on the primitive:
//!
//! - KV: Key validation happens in handlers (executor layer)
//! - JSON: Path parsing happens in handlers; payload validation in engine
//! - Event: event_type and payload validation happens in engine primitive
//! - State: Key validation in handlers; value serialization in engine
//! - Vector: Dimension validation in engine; collection name in handlers
//!
//! This means:
//! - Errors from different primitives have different error types and messages
//!   for similar kinds of invalid input
//! - Some validations can be bypassed if using lower-level APIs directly
//! - Error messages are inconsistent ("invalid key" vs "invalid input" vs engine errors)
//!
//! No behavioral test is written for this issue because the inconsistency is
//! purely structural -- it affects code organization and error message quality,
//! not correctness of the validation itself.

// This file intentionally contains no executable tests.
// The issue documents an architectural observation about validation layering.
//
// See:
//   - crates/executor/src/handlers/kv.rs:    validate_key() in handler
//   - crates/executor/src/handlers/json.rs:  parse_path() in handler
//   - crates/engine/src/primitives/event.rs: validate_payload() in engine
//   - crates/executor/src/handlers/state.rs: validate_key() in handler
//   - crates/executor/src/handlers/vector.rs: validate_not_internal_collection() in handler
