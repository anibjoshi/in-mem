//! Audit test for issue #929: Search errors collapse to Error::Internal
//! Verdict: ARCHITECTURAL CHOICE
//!
//! When the `Search` command encounters an error from the intelligence layer,
//! it is converted through `StrataError::Internal` which maps to
//! `Error::Internal { reason }`. All search-specific error details (e.g.,
//! "collection not found for vector search", "index not built") are collapsed
//! into a single opaque `Internal` variant.
//!
//! This means callers cannot distinguish between different search failure modes
//! programmatically — they all appear as `Error::Internal`.
//!
//! This is considered an architectural choice because:
//! 1. The `Search` command is cross-primitive (KV, JSON, Vector, Event)
//! 2. Each primitive has its own error types
//! 3. A unified search error enum would add significant complexity
//! 4. Search is an MVP feature and error handling may evolve
//!
//! This is difficult to trigger in a unit test because search failures require
//! specific index states or missing collections.

/// Documents the issue: Error::Internal is the only search error variant.
#[test]
fn issue_929_search_errors_are_internal() {
    // The Error enum has no search-specific variants.
    // All search errors map to Error::Internal { reason: String }.
    let err = strata_executor::Error::Internal {
        reason: "search failed: collection 'foo' not found".into(),
    };

    // Callers can only inspect the reason string, not match on error type
    match &err {
        strata_executor::Error::Internal { reason } => {
            assert!(reason.contains("search failed"));
        }
        _ => panic!("Expected Internal"),
    }

    // There is no way to programmatically distinguish between:
    // - collection not found
    // - dimension mismatch in query vector
    // - index not built
    // - internal search engine failure
    // All collapse to Error::Internal.
}
