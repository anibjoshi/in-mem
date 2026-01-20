//! M9 Comprehensive Test Suite
//!
//! This test suite verifies the M9 contract types implementation.
//! It is designed to be extended as Phase 2+ are implemented.
//!
//! ## Test Tiers
//!
//! - **Tier 1**: Type invariant tests (one file per contract type)
//! - **Tier 2**: Cross-type integration tests
//! - **Tier 3**: Backwards compatibility tests
//! - **Tier 4**: Migration validation tests
//! - **Tier 5**: Seven invariants conformance tests
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test --test m9_comprehensive
//! ```

// Test modules
mod test_utils;

// Tier 1: Type Invariant Tests
mod tier1_entity_ref_invariants;
mod tier1_primitive_type_invariants;
mod tier1_run_name_invariants;
mod tier1_timestamp_invariants;
mod tier1_version_invariants;
mod tier1_versioned_invariants;

// Tier 2: Cross-Type Integration Tests
mod tier2_cross_type_integration;

// Tier 3: Backwards Compatibility Tests
mod tier3_backwards_compatibility;

// Tier 4: Migration Validation Tests
mod tier4_migration_validation;

// Tier 5: Seven Invariants Conformance Tests
mod tier5_seven_invariants;

// Tier 6: Primitive Conformance Tests (Epic 64)
mod tier6_kv_conformance;
mod tier6_event_conformance;
