//! Audit test for issue #960: WAL segment header has no CRC
//! Verdict: ARCHITECTURAL CHOICE
//!
//! WAL (Write-Ahead Log) segment files have a binary header that includes
//! magic bytes, a format version, and a segment number. However, the header
//! itself has no CRC or checksum to protect its integrity.
//!
//! If the header bytes are corrupted on disk (e.g., due to a partial write,
//! bad sector, or filesystem bug), the WAL reader may:
//! - Misinterpret the format version, leading to incorrect parsing of entries
//! - Read the wrong segment number, causing out-of-order replay
//! - Fail with an opaque error rather than a clear "header corrupted" message
//!
//! Individual WAL entries do have CRC protection, so the corruption would
//! likely be detected when trying to read entries. The header-level CRC would
//! provide earlier and more precise detection.
//!
//! This is classified as an ARCHITECTURAL CHOICE because:
//! 1. The header is small (typically 16-32 bytes) and written atomically
//! 2. Entry-level CRCs provide a safety net for data integrity
//! 3. Adding a header CRC would change the on-disk format
//! 4. The magic bytes serve as a basic sanity check

/// Documents the architectural choice regarding WAL header integrity.
/// No runtime test is provided because triggering header corruption requires
/// direct file manipulation, which is outside the scope of the executor API.
#[test]
fn issue_960_wal_segment_header_no_crc_documented() {
    // WAL segment headers contain:
    //   - Magic bytes (basic format identification)
    //   - Format version (parsing strategy selector)
    //   - Segment number (ordering)
    //
    // Missing from headers:
    //   - CRC/checksum (integrity verification)
    //
    // Entry-level CRCs exist and catch most corruption scenarios.
    // A header CRC would catch corruption earlier but would require
    // a format change.
    //
    // ARCHITECTURAL CHOICE: The current design relies on magic bytes
    // for format identification and entry CRCs for data integrity.
    // Header-level CRC is a potential future improvement.
}
