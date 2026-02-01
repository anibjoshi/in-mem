//! Audit test for issue #947: Version::increment() panics at u64::MAX
//! Verdict: CONFIRMED BUG
//!
//! In core/contract/version.rs, the `increment()` method uses unchecked addition:
//!
//! ```ignore
//! pub const fn increment(&self) -> Self {
//!     match self {
//!         Version::Txn(v) => Version::Txn(*v + 1),
//!         Version::Sequence(v) => Version::Sequence(*v + 1),
//!         Version::Counter(v) => Version::Counter(*v + 1),
//!     }
//! }
//! ```
//!
//! When `v == u64::MAX`:
//! - In debug mode: panics with "attempt to add with overflow"
//! - In release mode: wraps to 0 due to Rust's default wrapping behavior
//!
//! Both behaviors are bugs:
//! - Panicking crashes the database
//! - Wrapping to 0 breaks version monotonicity (Invariant: versions are monotonically increasing)
//!
//! A `saturating_increment()` method exists and handles this correctly by capping at
//! u64::MAX, but production code uses `increment()` instead.

use strata_core::Version;

/// Demonstrates that Version::increment() panics in debug mode at u64::MAX.
///
/// This test uses catch_unwind to detect the overflow panic.
/// In release mode, the addition wraps to 0, which is also incorrect.
#[test]
fn issue_947_version_increment_panics_at_max_counter() {
    let v = Version::Counter(u64::MAX);

    let result = std::panic::catch_unwind(|| v.increment());

    match result {
        Err(_) => {
            // Panic confirmed in debug mode -- this is the bug.
            // Production code using increment() will crash if a version
            // ever reaches u64::MAX.
        }
        Ok(incremented) => {
            // Release mode: wraps to 0, which violates version monotonicity.
            // Version::Counter(u64::MAX).increment() should NOT produce
            // Version::Counter(0), as 0 < u64::MAX breaks ordering.
            assert_eq!(
                incremented,
                Version::Counter(0),
                "In release mode, increment wraps to 0 (also a bug)"
            );
        }
    }
}

/// Demonstrates the same bug with Txn variant.
#[test]
fn issue_947_version_increment_panics_at_max_txn() {
    let v = Version::Txn(u64::MAX);

    let result = std::panic::catch_unwind(|| v.increment());

    match result {
        Err(_) => {
            // Panic confirmed in debug mode
        }
        Ok(incremented) => {
            // Release mode wraps to 0
            assert_eq!(incremented, Version::Txn(0));
        }
    }
}

/// Demonstrates the same bug with Sequence variant.
#[test]
fn issue_947_version_increment_panics_at_max_sequence() {
    let v = Version::Sequence(u64::MAX);

    let result = std::panic::catch_unwind(|| v.increment());

    match result {
        Err(_) => {
            // Panic confirmed in debug mode
        }
        Ok(incremented) => {
            // Release mode wraps to 0
            assert_eq!(incremented, Version::Sequence(0));
        }
    }
}

/// Demonstrates that saturating_increment() correctly handles u64::MAX.
///
/// This method exists but is never used in production code.
/// It should replace increment() in all production call sites.
#[test]
fn issue_947_saturating_increment_handles_max() {
    let v_counter = Version::Counter(u64::MAX);
    let v_txn = Version::Txn(u64::MAX);
    let v_seq = Version::Sequence(u64::MAX);

    // saturating_increment caps at u64::MAX instead of overflowing
    assert_eq!(
        v_counter.saturating_increment(),
        Version::Counter(u64::MAX),
        "saturating_increment should cap at MAX"
    );
    assert_eq!(
        v_txn.saturating_increment(),
        Version::Txn(u64::MAX),
        "saturating_increment should cap at MAX"
    );
    assert_eq!(
        v_seq.saturating_increment(),
        Version::Sequence(u64::MAX),
        "saturating_increment should cap at MAX"
    );
}

/// Demonstrates that normal increment works correctly for non-MAX values.
#[test]
fn issue_947_normal_increment_works() {
    assert_eq!(Version::Counter(0).increment(), Version::Counter(1));
    assert_eq!(Version::Txn(42).increment(), Version::Txn(43));
    assert_eq!(Version::Sequence(100).increment(), Version::Sequence(101));

    // One below MAX is fine
    assert_eq!(
        Version::Counter(u64::MAX - 1).increment(),
        Version::Counter(u64::MAX)
    );
}
