# API Encapsulation Strategy for Strata

> **Status**: Proposal for Review
> **Author**: Architecture Team
> **Date**: 2026-01-24

---

## Executive Summary

Strata is a production-grade embedded database. This document proposes an API encapsulation strategy that:

1. Exposes only Substrate and Facade APIs to users
2. Hides all internal implementation details
3. Prevents users from bypassing safety mechanisms
4. Enables internal refactoring without breaking changes
5. Follows industry best practices from production databases

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Problem Statement](#2-problem-statement)
3. [Industry Best Practices](#3-industry-best-practices)
4. [Recommended Architecture](#4-recommended-architecture)
5. [Implementation Plan](#5-implementation-plan)
6. [Migration Guide](#6-migration-guide)
7. [Security Considerations](#7-security-considerations)
8. [Appendix: Rust Visibility Reference](#appendix-rust-visibility-reference)

---

## 1. Current State Analysis

### 1.1 Workspace Structure

```
strata/                          # Workspace root (no lib.rs)
├── crates/
│   ├── api/                     # Public API (Substrate + Facade)
│   ├── primitives/              # KVStore, EventLog, StateCell, etc.
│   ├── engine/                  # Database, Transaction, Recovery
│   ├── core/                    # Types, Contracts, Value
│   ├── storage/                 # LSM, WAL format, Snapshots
│   ├── concurrency/             # MVCC, Locks, Transaction context
│   ├── durability/              # WAL, Recovery, RunBundle
│   ├── search/                  # Full-text search, Hybrid search
│   └── wire/                    # Serialization, Protocol
```

### 1.2 Current Visibility

All crates currently expose their contents as `pub`:

```rust
// crates/primitives/src/lib.rs
pub use kv::{KVStore, KVTransaction};
pub use event_log::{Event, EventLog};
pub use state_cell::{State, StateCell};
// ... everything is public
```

### 1.3 Current Dependency Graph

```
User Code
    │
    ├── strata-api (intended public API)
    │       │
    │       ├── strata-primitives ← User can also depend on this directly!
    │       ├── strata-engine     ← User can also depend on this directly!
    │       └── ...
    │
    └── strata-primitives (BYPASSES API!) ← This is the problem
```

---

## 2. Problem Statement

### 2.1 Users Can Bypass the API

Currently, a user can add any internal crate as a dependency:

```toml
# User's Cargo.toml - bypasses all API safety!
[dependencies]
strata-primitives = { git = "..." }
strata-engine = { git = "..." }
```

This allows:
- Direct access to internal data structures
- Bypassing validation and safety checks
- Creating invalid states
- Breaking invariants

### 2.2 Internal Refactoring Breaks Users

If we change internal APIs, users who depend on them directly will break:

```rust
// We rename an internal struct
// Before: pub struct KVStore
// After:  pub struct KeyValueStore

// User's code breaks even though they "shouldn't" use this
```

### 2.3 No Clear API Boundary

Without clear encapsulation:
- Documentation mixes internal and public APIs
- Users don't know what's stable vs unstable
- Semver becomes meaningless for internals

---

## 3. Industry Best Practices

### 3.1 How Production Databases Handle This

| Database | Language | Approach |
|----------|----------|----------|
| **redb** | Rust | Single crate, strict `pub` only for API |
| **sled** | Rust | Single crate, `pub(crate)` for internals |
| **RocksDB** | C++ | Header files define public API, everything else internal |
| **SQLite** | C | Single `sqlite3.h` header is the entire public API |
| **LevelDB** | C++ | `include/` directory contains only public headers |
| **TiKV** | Rust | Multiple crates with `#[doc(hidden)]` and sealed traits |
| **DuckDB** | C++ | Public headers vs internal headers separation |

### 3.2 Common Patterns

#### Pattern A: Single Crate (redb, sled)
```
my-database/
├── src/
│   ├── lib.rs          # pub items = API
│   ├── internal/       # mod internal (private)
│   └── storage/        # mod storage (private)
```

**Pros**: Simple, clear boundary
**Cons**: Large crate, slower compilation

#### Pattern B: Workspace with Hidden Internals (TiKV)
```
my-database/
├── crates/
│   ├── my-database/    # Public crate, re-exports API
│   ├── internal-a/     # publish = false
│   └── internal-b/     # publish = false
```

**Pros**: Fast incremental compilation, modular
**Cons**: More complex setup

#### Pattern C: Public + Private Crates (common in Rust ecosystem)
```
my-database/
├── my-database/        # Public API crate
├── my-database-core/   # Internal, publish = false
└── my-database-proto/  # Internal, publish = false
```

---

## 4. Recommended Architecture

### 4.1 Overview

We recommend **Pattern B** adapted for Strata:

```
strata/                          # THE public crate (only thing users depend on)
├── src/lib.rs                   # Re-exports strata-api surfaces
├── crates/
│   ├── api/                     # publish = false, but re-exported by root
│   ├── primitives/              # publish = false, internal
│   ├── engine/                  # publish = false, internal
│   ├── core/                    # publish = false, internal
│   ├── storage/                 # publish = false, internal
│   ├── concurrency/             # publish = false, internal
│   ├── durability/              # publish = false, internal
│   ├── search/                  # publish = false, internal
│   └── wire/                    # publish = false, internal
```

### 4.2 The Public API (src/lib.rs)

```rust
//! # Strata
//!
//! A production-grade embedded database for AI agents.
//!
//! ## Quick Start
//!
//! ```rust
//! use strata::prelude::*;
//!
//! // Open a database
//! let db = Strata::builder()
//!     .path("./my-database")
//!     .open()?;
//!
//! // Use the Facade API (simple, Redis-like)
//! let facade = db.facade();
//! facade.set("key", json!({"count": 0}))?;
//! facade.incr("count")?;
//!
//! // Or use the Substrate API (advanced, explicit)
//! let substrate = db.substrate();
//! let run = substrate.create_run("my-agent-run")?;
//! substrate.kv_put(&run, "key", Value::Int(42))?;
//! ```
//!
//! ## API Layers
//!
//! Strata provides two API surfaces:
//!
//! - **Facade API**: Simple, Redis-like interface with auto-commit
//! - **Substrate API**: Full control with explicit runs, versions, transactions
//!
//! The Facade desugars to Substrate calls - no hidden magic.

#![warn(missing_docs)]
#![deny(unsafe_code)]

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

/// The Substrate API - explicit control over runs, versions, and transactions
pub mod substrate {
    pub use strata_api::substrate::*;
}

/// The Facade API - simple Redis-like interface
pub mod facade {
    pub use strata_api::facade::*;
}

/// Common types used across APIs
pub mod types {
    pub use strata_api::{ApiRunId, RunInfo, RunState};
    pub use strata_core::value::Value;
    pub use strata_core::contract::{Version, Versioned, Timestamp};
}

/// Convenient imports for common usage
pub mod prelude {
    pub use crate::Strata;
    pub use crate::facade::*;
    pub use crate::types::*;
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// The Strata database
///
/// This is the main entry point for using Strata. Create a database,
/// then access primitives through either the Facade or Substrate API.
///
/// # Example
///
/// ```rust
/// use strata::prelude::*;
///
/// let db = Strata::builder()
///     .path("./my-db")
///     .open()?;
///
/// // Access via Facade (simple)
/// db.facade().set("key", Value::Int(42))?;
///
/// // Access via Substrate (advanced)
/// let run = db.substrate().create_run("agent-1")?;
/// db.substrate().kv_put(&run, "key", Value::Int(42))?;
/// ```
pub use strata_engine::Database as Strata;

/// Database builder for configuration
pub use strata_engine::DatabaseBuilder as StrataBuilder;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Error types for Strata operations
pub mod error {
    pub use strata_core::error::{Error, Result};
    pub use strata_api::substrate::StrataError;
}
```

### 4.3 Internal Crate Configuration

Each internal crate's `Cargo.toml`:

```toml
[package]
name = "strata-primitives"
version.workspace = true
edition.workspace = true
publish = false  # <-- KEY: Cannot be published separately

# Internal crate - not for direct use
# Use the `strata` crate instead
```

### 4.4 Visibility Within Internal Crates

```rust
// crates/primitives/src/kv.rs

/// KVStore implementation
///
/// NOTE: This is an internal type. Use `strata::substrate::KVStore` trait instead.
#[doc(hidden)]  // Hidden from docs but accessible to sibling crates
pub struct KVStoreImpl {
    // ...
}

// Truly internal items use pub(crate)
pub(crate) struct InternalCache {
    // ...
}

// Items only needed by specific sibling crates
pub(in crate::engine) fn recovery_hook() {
    // Only engine crate can call this
}
```

### 4.5 Sealed Traits

Prevent users from implementing internal traits:

```rust
// In strata-api

/// Marker module for sealed traits
mod private {
    /// Sealed trait marker - cannot be implemented outside this crate
    pub trait Sealed {}
}

/// A Strata primitive (KV, Event, State, Json, Vector, Run)
///
/// This trait is sealed and cannot be implemented outside Strata.
pub trait Primitive: private::Sealed {
    /// The primitive type identifier
    fn primitive_type(&self) -> PrimitiveType;
}

// Implement Sealed for our types (users can't do this)
impl private::Sealed for KVStoreImpl {}
impl Primitive for KVStoreImpl {
    fn primitive_type(&self) -> PrimitiveType {
        PrimitiveType::Kv
    }
}
```

---

## 5. Implementation Plan

### Phase 1: Add `publish = false` to Internal Crates

**Files to modify:**
- `crates/core/Cargo.toml`
- `crates/storage/Cargo.toml`
- `crates/concurrency/Cargo.toml`
- `crates/durability/Cargo.toml`
- `crates/primitives/Cargo.toml`
- `crates/engine/Cargo.toml`
- `crates/search/Cargo.toml`
- `crates/wire/Cargo.toml`
- `crates/api/Cargo.toml`

**Change:**
```toml
[package]
name = "strata-xxx"
publish = false  # Add this line
```

**Impact**: None - just prevents accidental publishing

### Phase 2: Create Public Entry Point

**Files to create:**
- `src/lib.rs` (the public API surface)

**Impact**: Provides single entry point for users

### Phase 3: Add `#[doc(hidden)]` to Internal Items

**Files to modify:**
- All `pub` items in internal crates that must remain `pub` for sibling access

**Change:**
```rust
#[doc(hidden)]
pub struct InternalThing { ... }
```

**Impact**: Cleans up documentation

### Phase 4: Implement Sealed Traits

**Files to modify:**
- `crates/api/src/substrate/mod.rs`

**Impact**: Prevents external trait implementations

### Phase 5: Update Documentation

**Files to modify:**
- `README.md`
- `docs/GETTING_STARTED.md`

**Change**: Update examples to use `strata` crate only

---

## 6. Migration Guide

### For Internal Development

No changes needed. Internal crates continue to depend on each other normally:

```toml
# crates/engine/Cargo.toml
[dependencies]
strata-primitives = { path = "../primitives" }  # Still works
```

### For Users (if any exist)

Before:
```toml
[dependencies]
strata-api = { git = "..." }
strata-primitives = { git = "..." }  # Direct access
```

After:
```toml
[dependencies]
strata = { git = "..." }  # Single dependency
```

Code changes:
```rust
// Before
use strata_api::substrate::KVStore;
use strata_primitives::KVStore as RawKV;  // No longer works

// After
use strata::substrate::KVStore;  // This is the only way
```

---

## 7. Security Considerations

### 7.1 What This Prevents

| Attack Vector | Mitigation |
|--------------|------------|
| Bypassing validation | Users can't access internal APIs |
| Creating invalid states | Only safe APIs exposed |
| Breaking invariants | Internal consistency maintained |
| Accessing raw storage | Storage layer hidden |
| WAL manipulation | Durability layer hidden |

### 7.2 What This Does NOT Prevent

| Still Possible | Reason |
|----------------|--------|
| Memory inspection | Rust can't prevent this at runtime |
| Unsafe code in user crate | User's responsibility |
| Fork and modify | Open source nature |

### 7.3 Defense in Depth

Even with encapsulation, maintain internal invariant checks:

```rust
// In internal code, still validate
pub(crate) fn write_to_wal(entry: &WalEntry) -> Result<()> {
    // Even though this is internal, still validate
    debug_assert!(entry.is_valid(), "Invalid WAL entry from internal caller");
    // ...
}
```

---

## Appendix: Rust Visibility Reference

### Visibility Modifiers

| Modifier | Scope | Use Case |
|----------|-------|----------|
| (none) | Current module only | True implementation details |
| `pub(self)` | Same as none | Explicit private |
| `pub(super)` | Parent module | Shared within a module tree |
| `pub(crate)` | Current crate | Internal API within crate |
| `pub(in path)` | Specific ancestor | Fine-grained sibling access |
| `pub` | Everywhere | True public API |

### `#[doc(hidden)]`

```rust
/// This item is public but not documented
#[doc(hidden)]
pub struct InternalButAccessible;
```

- Item is still `pub` (accessible)
- Hidden from rustdoc output
- IDE may still show it
- Use for items that must be `pub` for technical reasons

### Sealed Trait Pattern

```rust
mod private {
    pub trait Sealed {}
}

pub trait MyTrait: private::Sealed {
    fn method(&self);
}

// Only we can implement MyTrait because only we can implement Sealed
struct MyType;
impl private::Sealed for MyType {}
impl MyTrait for MyType {
    fn method(&self) {}
}
```

---

## Decision Checklist

- [ ] Approve `publish = false` for all internal crates
- [ ] Approve public API surface in `src/lib.rs`
- [ ] Approve sealed trait pattern for primitives
- [ ] Approve `#[doc(hidden)]` for internal-but-accessible items
- [ ] Approve documentation updates

---

## References

1. [Rust API Guidelines - Necessities](https://rust-lang.github.io/api-guidelines/necessities.html)
2. [Sealed Traits in Rust](https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/)
3. [redb source code](https://github.com/cberner/redb)
4. [sled source code](https://github.com/spacejam/sled)
5. [TiKV crate organization](https://github.com/tikv/tikv)
