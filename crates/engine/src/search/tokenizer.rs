//! Text tokenizer for search operations
//!
//! Pipeline: lowercase → split on non-alphanumeric → filter short tokens
//!           → remove stopwords → Porter stem

use super::stemmer;

/// Standard English stopwords (Lucene's default set).
///
/// These high-frequency words carry little discriminative value for BM25
/// and are filtered out during tokenization.
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it",
    "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these",
    "they", "this", "to", "was", "will", "with",
];

/// Check if a token is a stopword.
#[inline]
fn is_stopword(token: &str) -> bool {
    // Linear scan is fast for ~33 entries (all < cache line).
    STOPWORDS.contains(&token)
}

/// Tokenize text into searchable terms.
///
/// Pipeline:
/// 1. Lowercase
/// 2. Split on non-alphanumeric characters
/// 3. Filter tokens shorter than 2 characters
/// 4. Remove stopwords
/// 5. Porter-stem each token
///
/// # Example
///
/// ```
/// use strata_engine::search::tokenizer::tokenize;
///
/// let tokens = tokenize("The Quick Brown Foxes");
/// assert_eq!(tokens, vec!["quick", "brown", "fox"]);
/// ```
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .filter(|s| !is_stopword(s))
        .map(|s| stemmer::stem(s))
        .collect()
}

/// Tokenize and deduplicate for query processing.
///
/// # Example
///
/// ```
/// use strata_engine::search::tokenizer::tokenize_unique;
///
/// let tokens = tokenize_unique("testing tests TESTS");
/// assert_eq!(tokens, vec!["test"]);
/// ```
pub fn tokenize_unique(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    tokenize(text)
        .into_iter()
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello, World!");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_filters_short() {
        // "I" and "a" filtered (< 2 chars); "a" is also a stopword
        let tokens = tokenize("I am a test");
        assert_eq!(tokens, vec!["am", "test"]);
    }

    #[test]
    fn test_tokenize_numbers() {
        let tokens = tokenize("test123 foo456bar");
        assert_eq!(tokens, vec!["test123", "foo456bar"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_only_punctuation() {
        let tokens = tokenize("...---...");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_unique() {
        // "test", "test", "test" all stem to "test" → deduplicated
        let tokens = tokenize_unique("test test TEST");
        assert_eq!(tokens, vec!["test"]);
    }

    #[test]
    fn test_tokenize_unique_preserves_order() {
        let tokens = tokenize_unique("apple banana apple cherry");
        assert_eq!(tokens, vec!["appl", "banana", "cherri"]);
    }

    // ------------------------------------------------------------------
    // Stopword tests
    // ------------------------------------------------------------------

    #[test]
    fn test_stopwords_removed() {
        let tokens = tokenize("the quick and the dead");
        // "the" (x2) and "and" are stopwords
        assert_eq!(tokens, vec!["quick", "dead"]);
    }

    #[test]
    fn test_all_stopwords() {
        let tokens = tokenize("the a an is are was");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_stopwords_case_insensitive() {
        let tokens = tokenize("The AND Not");
        // All are stopwords after lowercasing
        assert!(tokens.is_empty());
    }

    // ------------------------------------------------------------------
    // Stemming integration tests
    // ------------------------------------------------------------------

    #[test]
    fn test_stemming_applied() {
        let tokens = tokenize("running quickly");
        assert_eq!(tokens, vec!["run", "quickli"]);
    }

    #[test]
    fn test_stemming_morphological_variants() {
        // "treatments" and "treatment" should produce the same stem
        let t1 = tokenize("treatments");
        let t2 = tokenize("treatment");
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_stemming_plurals() {
        let tokens = tokenize("infections diseases patients");
        assert_eq!(tokens, vec!["infect", "diseas", "patient"]);
    }

    #[test]
    fn test_full_pipeline() {
        // Combines stopword removal + stemming
        let tokens = tokenize("The treatment of bacterial infections in patients");
        // "the", "of", "in" are stopwords
        assert_eq!(tokens, vec!["treatment", "bacteri", "infect", "patient"]);
    }

    #[test]
    fn test_unique_after_stemming() {
        // "run", "running", "runs" all stem to "run"
        let tokens = tokenize_unique("run running runs");
        assert_eq!(tokens, vec!["run"]);
    }
}
