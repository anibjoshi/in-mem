# WAL System Architecture

## Overview

The strata-durability crate implements a Write-Ahead Log (WAL) for durability.

## Main WAL System (wal.rs, encoding.rs, recovery.rs)

**Purpose**: Versioned key-value operations with MVCC support

**Used by**:
- strata-concurrency (transaction management, MVCC)
- strata-engine (database durability layer)
- strata-primitives (vector operations, run index)

**Key Types**:
- `WAL` - File handle with append/read operations
- `WALEntry` - Enum with BeginTxn, Write, Delete, CommitTxn, AbortTxn, Vector*, JSON*, etc.
- `DurabilityMode` - Strict/Batched/None fsync modes

**Features**:
- Rich `Key` type (namespace + type_tag + user_key)
- `Value` enum (Int, String, Bytes, etc.)
- Version tracking per entry (for MVCC)
- u64 transaction IDs
- Run ID tracking per entry

## WAL Entry Type Registry (wal_entry_types.rs)

The `WalEntryType` enum provides a standardized registry of entry types
organized by primitive ranges:

| Range | Primitive | Description |
|-------|-----------|-------------|
| 0x00-0x0F | Core | Transaction control (commit, abort, snapshot) |
| 0x10-0x1F | KV | Key-value operations |
| 0x20-0x2F | JSON | JSON document operations |
| 0x30-0x3F | Event | Event log operations |
| 0x40-0x4F | State | State cell operations |
| 0x60-0x6F | Run | Run lifecycle operations |
| 0x70-0x7F | Vector | Vector primitive operations |

This enum is used by primitives implementing the `PrimitiveStorageExt` trait
for WAL replay via `apply_wal_entry(entry_type: u8, payload: &[u8])`.

## Related Components

- **Snapshots**: `snapshot.rs`, `snapshot_types.rs` for point-in-time persistence
- **RunBundle**: `run_bundle/` for portable execution artifacts
- **Recovery**: `recovery.rs` for WAL replay after crash
