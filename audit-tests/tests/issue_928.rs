//! Audit test for issue #928: Storage error source chain discarded in convert.rs
//! Verdict: ARCHITECTURAL CHOICE
//!
//! In the `From<StrataError> for Error` conversion (convert.rs), the
//! `StrataError::Storage { message, source }` variant only keeps `message`,
//! discarding `source`. This means the underlying I/O error (e.g., an
//! `std::io::Error`) is lost and cannot be inspected by callers.
//!
//! The conversion is:
//!
//! ```ignore
//! StrataError::Storage { message, .. } => Error::Io { reason: message },
//! ```
//!
//! The `source` field (which contains the original `std::io::Error` or other
//! error) is dropped. This is an architectural choice because `Error` is
//! `Clone + Serialize + Deserialize`, and `std::io::Error` is neither `Clone`
//! nor `Serialize`. Preserving the source chain would require a different
//! error strategy (e.g., `Arc<dyn Error>` or stringifying the full chain).
//!
//! This is difficult to test directly because triggering a genuine storage
//! error in an ephemeral database is non-trivial. The test below documents
//! the issue and confirms the conversion behavior.

/// Documents the issue. The `Error::Io` variant only holds a `reason: String`,
/// so any upstream error source chain is inherently lost.
#[test]
fn issue_928_error_io_variant_has_no_source_chain() {
    // The executor Error::Io variant is:
    //   Io { reason: String }
    //
    // It implements thiserror::Error, but there is no `#[source]` or
    // `#[from]` on the reason field, so `.source()` always returns None.
    let err = strata_executor::Error::Io {
        reason: "disk full".into(),
    };

    // The error implements std::error::Error
    let std_err: &dyn std::error::Error = &err;

    // source() returns None because there is no chained error
    assert!(
        std_err.source().is_none(),
        "Error::Io has no source chain — the original storage error is discarded"
    );

    // The only information preserved is the message string
    assert_eq!(err.to_string(), "I/O error: disk full");
}

/// Confirms that StrataError::Storage conversion drops the source.
#[test]
fn issue_928_strata_storage_error_source_dropped_on_conversion() {
    use strata_core::StrataError;

    // Create a StrataError::Storage with both message and source
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let strata_err = StrataError::Storage {
        message: "failed to write".into(),
        source: Some(Box::new(io_err)),
    };

    // Convert to executor Error
    let exec_err: strata_executor::Error = strata_err.into();

    // Only the message survives; the io::Error source is gone
    match exec_err {
        strata_executor::Error::Io { reason } => {
            assert_eq!(reason, "failed to write");
            // The original "access denied" io::Error is not accessible
        }
        other => panic!("Expected Io variant, got: {:?}", other),
    }
}
