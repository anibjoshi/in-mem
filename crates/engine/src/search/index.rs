//! Optional inverted index for fast keyword search
//!
//! This module provides:
//! - InvertedIndex with posting lists
//! - Enable/disable functionality
//! - Synchronous index updates on commit
//! - Version watermark for consistency
//!
//! See `docs/architecture/M6_ARCHITECTURE.md` for authoritative specification.
//!
//! # Architectural Rules
//!
//! - Rule 1: No Data Movement - index stores integer doc IDs, not content
//! - Rule 5: Zero Overhead When Disabled - NOOP when disabled
//!
//! # Memory Efficiency
//!
//! PostingEntry uses a compact `u32` doc ID (12 bytes, Copy) instead of
//! cloning a full EntityRef (~87 bytes with heap allocation) per posting.
//! A single bidirectional `DocIdMap` holds one copy of each EntityRef,
//! reducing memory from O(terms × docs) to O(docs) for EntityRef storage.
//!
//! # Usage
//!
//! Indexing is OPTIONAL. Search works without it (via full scan).
//! When enabled, search uses the index for candidate lookup.

use super::tokenizer::tokenize;
use super::types::EntityRef;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use strata_core::types::BranchId;

// ============================================================================
// PostingEntry
// ============================================================================

/// Entry in a posting list
///
/// Compact 12-byte, Copy struct. Uses an integer doc ID instead of a cloned
/// EntityRef to avoid 87 bytes of heap allocation per posting entry.
/// Resolve to EntityRef via `InvertedIndex::resolve_doc_id()`.
#[derive(Debug, Clone, Copy)]
pub struct PostingEntry {
    /// Integer document identifier (resolve via InvertedIndex::resolve_doc_id)
    pub doc_id: u32,
    /// Term frequency in this document
    pub tf: u32,
    /// Document length in tokens
    pub doc_len: u32,
}

impl PostingEntry {
    /// Create a new posting entry
    pub fn new(doc_id: u32, tf: u32, doc_len: u32) -> Self {
        PostingEntry {
            doc_id,
            tf,
            doc_len,
        }
    }
}

// ============================================================================
// ScoredDocId
// ============================================================================

/// Result of in-index BM25 scoring: doc_id + score.
///
/// Returned by `InvertedIndex::score_top_k()` to avoid exposing
/// posting list internals. Consumers resolve the `doc_id` back to
/// an `EntityRef` via `InvertedIndex::resolve_doc_id()`.
#[derive(Debug, Clone, Copy)]
pub struct ScoredDocId {
    /// Integer document identifier (resolve via InvertedIndex::resolve_doc_id)
    pub doc_id: u32,
    /// BM25 relevance score
    pub score: f32,
}

// ============================================================================
// DocIdMap
// ============================================================================

/// Bidirectional mapping between EntityRef and compact u32 doc IDs.
///
/// Stores exactly one copy of each EntityRef (not 60× per term).
/// Memory at 5.4M docs: ~918 MB (vs 28 GB with per-posting clones).
struct DocIdMap {
    /// doc_id -> EntityRef (append-only, indexed by doc_id)
    id_to_ref: RwLock<Vec<EntityRef>>,
    /// EntityRef -> doc_id (for O(1) lookup on index/remove)
    ref_to_id: DashMap<EntityRef, u32>,
}

impl DocIdMap {
    fn new() -> Self {
        Self {
            id_to_ref: RwLock::new(Vec::new()),
            ref_to_id: DashMap::new(),
        }
    }

    /// Get or assign a doc_id for the given EntityRef.
    fn get_or_insert(&self, doc_ref: &EntityRef) -> u32 {
        // Fast path: already assigned
        if let Some(id) = self.ref_to_id.get(doc_ref) {
            return *id;
        }

        // Slow path: assign new ID
        let mut vec = self.id_to_ref.write().unwrap();
        // Double-check after acquiring write lock
        if let Some(id) = self.ref_to_id.get(doc_ref) {
            return *id;
        }
        let id = vec.len() as u32;
        vec.push(doc_ref.clone());
        self.ref_to_id.insert(doc_ref.clone(), id);
        id
    }

    /// Look up a doc_id, returning None if the EntityRef is unknown.
    fn get(&self, doc_ref: &EntityRef) -> Option<u32> {
        self.ref_to_id.get(doc_ref).map(|r| *r)
    }

    /// Resolve a doc_id back to its EntityRef.
    fn resolve(&self, doc_id: u32) -> Option<EntityRef> {
        let vec = self.id_to_ref.read().unwrap();
        vec.get(doc_id as usize).cloned()
    }

    fn clear(&self) {
        self.id_to_ref.write().unwrap().clear();
        self.ref_to_id.clear();
    }
}

// ============================================================================
// PostingList
// ============================================================================

/// List of documents containing a term
#[derive(Debug, Clone, Default)]
pub struct PostingList {
    /// Document entries
    pub entries: Vec<PostingEntry>,
}

impl PostingList {
    /// Create a new empty posting list
    pub fn new() -> Self {
        PostingList { entries: vec![] }
    }

    /// Add an entry to the posting list
    pub fn add(&mut self, entry: PostingEntry) {
        self.entries.push(entry);
    }

    /// Remove entries matching a doc_id
    pub fn remove_by_id(&mut self, doc_id: u32) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| e.doc_id != doc_id);
        before - self.entries.len()
    }

    /// Number of documents containing this term
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if posting list is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ============================================================================
// InvertedIndex
// ============================================================================

/// Inverted index for fast keyword search
///
/// CRITICAL: This is OPTIONAL. Search works without it (via scan).
/// When disabled, all operations are NOOP (zero overhead).
///
/// # Thread Safety
///
/// Uses DashMap for concurrent access. Multiple readers/writers supported.
///
/// # Version Watermark
///
/// The version field tracks index state for consistency checking.
/// Incremented on every update operation.
pub struct InvertedIndex {
    /// Term -> PostingList mapping
    postings: DashMap<String, PostingList>,

    /// Term -> document frequency
    doc_freqs: DashMap<String, usize>,

    /// Total documents indexed
    total_docs: AtomicUsize,

    /// Whether index is enabled
    enabled: AtomicBool,

    /// Version watermark for consistency
    version: AtomicU64,

    /// Sum of all document lengths (for average calculation)
    total_doc_len: AtomicUsize,

    /// doc_id -> document length (indexed by u32 doc_id, 4 bytes per doc)
    /// Replaces DashMap<EntityRef, u32> which cost ~578 MB at 5.4M docs.
    doc_lengths: RwLock<Vec<Option<u32>>>,

    /// Bidirectional EntityRef <-> u32 mapping (one copy per doc, not per term)
    doc_id_map: DocIdMap,
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl InvertedIndex {
    /// Create a new disabled index
    pub fn new() -> Self {
        InvertedIndex {
            postings: DashMap::new(),
            doc_freqs: DashMap::new(),
            total_docs: AtomicUsize::new(0),
            enabled: AtomicBool::new(false),
            version: AtomicU64::new(0),
            total_doc_len: AtomicUsize::new(0),
            doc_lengths: RwLock::new(Vec::new()),
            doc_id_map: DocIdMap::new(),
        }
    }

    // ========================================================================
    // Enable/Disable
    // ========================================================================

    /// Check if index is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Enable the index
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    /// Disable the index
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    /// Clear all index data
    ///
    /// Does NOT change enabled state.
    pub fn clear(&self) {
        self.postings.clear();
        self.doc_freqs.clear();
        self.doc_lengths.write().unwrap().clear();
        self.doc_id_map.clear();
        self.total_docs.store(0, Ordering::Relaxed);
        self.total_doc_len.store(0, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Release);
    }

    // ========================================================================
    // Version Watermark
    // ========================================================================

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Check if index is at least at given version
    pub fn is_at_version(&self, min_version: u64) -> bool {
        self.version.load(Ordering::Acquire) >= min_version
    }

    /// Wait for index to reach a version (with timeout)
    ///
    /// Returns true if version was reached, false on timeout.
    pub fn wait_for_version(&self, version: u64, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            if self.version.load(Ordering::Acquire) >= version {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::yield_now();
        }
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get total number of indexed documents
    ///
    /// Uses Acquire ordering to ensure visibility of updates from other threads.
    pub fn total_docs(&self) -> usize {
        self.total_docs.load(Ordering::Acquire)
    }

    /// Get document frequency for a term
    pub fn doc_freq(&self, term: &str) -> usize {
        self.doc_freqs.get(term).map(|r| *r).unwrap_or(0)
    }

    /// Get average document length
    ///
    /// Uses Acquire ordering to ensure consistent visibility of both counters.
    pub fn avg_doc_len(&self) -> f32 {
        let total = self.total_docs.load(Ordering::Acquire);
        if total == 0 {
            return 0.0;
        }
        self.total_doc_len.load(Ordering::Acquire) as f32 / total as f32
    }

    /// Compute IDF for a term
    ///
    /// Uses standard IDF formula with smoothing:
    /// IDF(t) = ln((N - df + 0.5) / (df + 0.5) + 1)
    ///
    /// Uses Acquire ordering to ensure visibility of document count updates.
    pub fn compute_idf(&self, term: &str) -> f32 {
        let n = self.total_docs.load(Ordering::Acquire) as f32;
        let df = self.doc_freq(term) as f32;
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    // ========================================================================
    // Doc ID Resolution
    // ========================================================================

    /// Resolve a compact u32 doc_id back to its EntityRef.
    ///
    /// Used by search consumers (e.g., KVStore::search) to map posting
    /// entries back to the original document references.
    pub fn resolve_doc_id(&self, doc_id: u32) -> Option<EntityRef> {
        self.doc_id_map.resolve(doc_id)
    }

    // ========================================================================
    // In-Index BM25 Scoring
    // ========================================================================

    /// Score documents using BM25 entirely within the index.
    ///
    /// Key optimizations over the previous approach:
    /// - Posting lists iterated by DashMap reference (zero clone)
    /// - doc_id_map RwLock acquired ONCE for entire search
    /// - Scores accumulated by u32 doc_id (not String keys)
    /// - Top-k extracted via sort + truncate
    /// - No EntityRef resolution except for final top-k (done by caller)
    pub fn score_top_k(
        &self,
        query_terms: &[String],
        branch_id: &BranchId,
        k: usize,
        scorer_k1: f32,
        scorer_b: f32,
    ) -> Vec<ScoredDocId> {
        if !self.is_enabled() || query_terms.is_empty() || k == 0 {
            return Vec::new();
        }

        let total_docs = self.total_docs.load(Ordering::Acquire) as f32;
        let avg_doc_len = self.avg_doc_len().max(1.0);

        // Precompute IDF for each query term
        let term_idfs: Vec<(&str, f32)> = query_terms
            .iter()
            .map(|t| {
                let df = self.doc_freq(t) as f32;
                let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                (t.as_str(), idf)
            })
            .collect();

        // Acquire doc_id_map read lock ONCE for the entire search
        let id_to_ref = self.doc_id_map.id_to_ref.read().unwrap();

        // Accumulate BM25 scores by doc_id (u32 key = fast hashing)
        let mut scores: HashMap<u32, f32> = HashMap::new();

        for (term, idf) in &term_idfs {
            // Iterate posting list by reference — NO clone
            if let Some(posting_list) = self.postings.get(*term) {
                for entry in &posting_list.entries {
                    // Branch filter using held lock — no per-entry lock acquisition
                    match id_to_ref.get(entry.doc_id as usize) {
                        Some(entity_ref) if entity_ref.branch_id() == *branch_id => {}
                        _ => continue,
                    }

                    // BM25 partial score for this term
                    let tf = entry.tf as f32;
                    let dl = entry.doc_len as f32;
                    let tf_component = (tf * (scorer_k1 + 1.0))
                        / (tf + scorer_k1 * (1.0 - scorer_b + scorer_b * dl / avg_doc_len));
                    let partial = idf * tf_component;

                    *scores.entry(entry.doc_id).or_insert(0.0) += partial;
                }
            }
        }

        drop(id_to_ref); // Release lock before sorting

        if scores.is_empty() {
            return Vec::new();
        }

        // Collect, sort descending by score, and take top-k
        let mut result: Vec<ScoredDocId> = scores
            .into_iter()
            .map(|(doc_id, score)| ScoredDocId { doc_id, score })
            .collect();
        result.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result.truncate(k);
        result
    }

    // ========================================================================
    // Index Updates
    // ========================================================================

    /// Index a document
    ///
    /// NOOP if index is disabled.
    /// If document is already indexed, removes old version first (fixes #609).
    pub fn index_document(&self, doc_ref: &EntityRef, text: &str, _ts_micros: Option<u64>) {
        if !self.is_enabled() {
            return; // Zero overhead when disabled
        }

        // Get or assign a compact doc_id
        let doc_id = self.doc_id_map.get_or_insert(doc_ref);

        // Fix #609: Check if document already indexed, remove first to prevent double-counting
        {
            let lengths = self.doc_lengths.read().unwrap();
            if lengths.get(doc_id as usize).copied().flatten().is_some() {
                drop(lengths);
                self.remove_document(doc_ref);
            }
        }

        let tokens = tokenize(text);
        let doc_len = tokens.len() as u32;

        // Count term frequencies
        let mut tf_map: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *tf_map.entry(token.clone()).or_insert(0) += 1;
        }

        // Update posting lists
        for (term, tf) in tf_map {
            let entry = PostingEntry::new(doc_id, tf, doc_len);

            self.postings.entry(term.clone()).or_default().add(entry);

            self.doc_freqs
                .entry(term)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        // Track document length for proper removal (fixes #608)
        {
            let mut lengths = self.doc_lengths.write().unwrap();
            let idx = doc_id as usize;
            if idx >= lengths.len() {
                lengths.resize(idx + 1, None);
            }
            lengths[idx] = Some(doc_len);
        }

        self.total_docs.fetch_add(1, Ordering::Relaxed);
        self.total_doc_len
            .fetch_add(doc_len as usize, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Release);
    }

    /// Remove a document from the index
    ///
    /// NOOP if index is disabled.
    /// Properly decrements total_doc_len using tracked document length (fixes #608).
    pub fn remove_document(&self, doc_ref: &EntityRef) {
        if !self.is_enabled() {
            return;
        }

        // Resolve EntityRef -> doc_id
        let doc_id = match self.doc_id_map.get(doc_ref) {
            Some(id) => id,
            None => return, // Not indexed
        };

        // Fix #608: Get document length before removal for proper total_doc_len update
        let doc_len = {
            let mut lengths = self.doc_lengths.write().unwrap();
            let idx = doc_id as usize;
            if idx < lengths.len() {
                lengths[idx].take()
            } else {
                None
            }
        };

        let mut removed = false;

        for mut entry in self.postings.iter_mut() {
            let count = entry.remove_by_id(doc_id);
            if count > 0 {
                removed = true;
                let term = entry.key().clone();
                self.doc_freqs
                    .entry(term)
                    .and_modify(|c| *c = c.saturating_sub(count));
            }
        }

        if removed || doc_len.is_some() {
            self.total_docs.fetch_sub(1, Ordering::Relaxed);
            // Fix #608: Properly decrement total_doc_len using tracked length
            if let Some(len) = doc_len {
                self.total_doc_len
                    .fetch_sub(len as usize, Ordering::Relaxed);
            }
            self.version.fetch_add(1, Ordering::Release);
        }
    }

    // ========================================================================
    // Query
    // ========================================================================

    /// Lookup documents containing a term
    ///
    /// Returns None if term not found or index disabled.
    pub fn lookup(&self, term: &str) -> Option<PostingList> {
        if !self.is_enabled() {
            return None;
        }
        self.postings.get(term).map(|r| r.clone())
    }

    /// Get all terms in the index
    pub fn terms(&self) -> Vec<String> {
        self.postings.iter().map(|r| r.key().clone()).collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use strata_core::types::BranchId;

    fn test_doc_ref(name: &str) -> EntityRef {
        let branch_id = BranchId::new();
        EntityRef::Kv {
            branch_id,
            key: name.to_string(),
        }
    }

    #[test]
    fn test_index_disabled_by_default() {
        let index = InvertedIndex::new();
        assert!(!index.is_enabled());
    }

    #[test]
    fn test_index_enable_disable() {
        let index = InvertedIndex::new();

        index.enable();
        assert!(index.is_enabled());

        index.disable();
        assert!(!index.is_enabled());
    }

    #[test]
    fn test_index_noop_when_disabled() {
        let index = InvertedIndex::new();
        let doc_ref = test_doc_ref("test");

        // Should be NOOP when disabled
        index.index_document(&doc_ref, "hello world", None);

        assert_eq!(index.total_docs(), 0);
        assert!(index.lookup("hello").is_none());
    }

    #[test]
    fn test_index_document_when_enabled() {
        let index = InvertedIndex::new();
        index.enable();

        let doc_ref = test_doc_ref("test");
        index.index_document(&doc_ref, "hello world test", None);

        assert_eq!(index.total_docs(), 1);
        assert_eq!(index.doc_freq("hello"), 1);
        assert_eq!(index.doc_freq("world"), 1);
        assert_eq!(index.doc_freq("test"), 1);

        let postings = index.lookup("hello").unwrap();
        assert_eq!(postings.len(), 1);
        assert_eq!(postings.entries[0].tf, 1);
        assert_eq!(postings.entries[0].doc_len, 3);
    }

    #[test]
    fn test_index_multiple_documents() {
        let index = InvertedIndex::new();
        index.enable();

        let doc1 = test_doc_ref("doc1");
        let doc2 = test_doc_ref("doc2");

        index.index_document(&doc1, "hello world", None);
        index.index_document(&doc2, "hello planet", None);

        assert_eq!(index.total_docs(), 2);
        assert_eq!(index.doc_freq("hello"), 2); // In both docs
        assert_eq!(index.doc_freq("world"), 1); // Only in doc1
        assert_eq!(index.doc_freq("planet"), 1); // Only in doc2

        let postings = index.lookup("hello").unwrap();
        assert_eq!(postings.len(), 2);
    }

    #[test]
    fn test_index_term_frequency() {
        let index = InvertedIndex::new();
        index.enable();

        let doc_ref = test_doc_ref("test");
        index.index_document(&doc_ref, "hello hello hello world", None);

        let postings = index.lookup("hello").unwrap();
        assert_eq!(postings.entries[0].tf, 3); // "hello" appears 3 times

        let postings = index.lookup("world").unwrap();
        assert_eq!(postings.entries[0].tf, 1);
    }

    #[test]
    fn test_remove_document() {
        let index = InvertedIndex::new();
        index.enable();

        let doc1 = test_doc_ref("doc1");
        let doc2 = test_doc_ref("doc2");

        index.index_document(&doc1, "hello world", None);
        index.index_document(&doc2, "hello there", None);

        assert_eq!(index.total_docs(), 2);

        index.remove_document(&doc1);

        assert_eq!(index.total_docs(), 1);
        assert_eq!(index.doc_freq("hello"), 1);
        assert_eq!(index.doc_freq("world"), 0);
    }

    #[test]
    fn test_clear() {
        let index = InvertedIndex::new();
        index.enable();

        let doc_ref = test_doc_ref("test");
        index.index_document(&doc_ref, "hello world", None);

        let v1 = index.version();
        index.clear();
        let v2 = index.version();

        assert_eq!(index.total_docs(), 0);
        assert!(index.lookup("hello").is_none());
        assert!(v2 > v1); // Version incremented
    }

    #[test]
    fn test_version_increment() {
        let index = InvertedIndex::new();
        index.enable();

        let v0 = index.version();

        let doc_ref = test_doc_ref("test");
        index.index_document(&doc_ref, "hello", None);
        let v1 = index.version();

        index.remove_document(&doc_ref);
        let v2 = index.version();

        assert!(v1 > v0);
        assert!(v2 > v1);
    }

    #[test]
    fn test_compute_idf() {
        let index = InvertedIndex::new();
        index.enable();

        // Add 10 documents, "common" in all, "rare" in 1
        for i in 0..10 {
            let doc_ref = test_doc_ref(&format!("doc{}", i));
            if i == 0 {
                index.index_document(&doc_ref, "common rare", None);
            } else {
                index.index_document(&doc_ref, "common", None);
            }
        }

        let idf_common = index.compute_idf("common");
        let idf_rare = index.compute_idf("rare");

        // Rare terms should have higher IDF
        assert!(idf_rare > idf_common);
    }

    #[test]
    fn test_avg_doc_len() {
        let index = InvertedIndex::new();
        index.enable();

        let doc1 = test_doc_ref("doc1");
        let doc2 = test_doc_ref("doc2");

        index.index_document(&doc1, "one two", None); // 2 tokens
        index.index_document(&doc2, "one two three four", None); // 4 tokens

        // Average: (2 + 4) / 2 = 3.0
        assert!((index.avg_doc_len() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_wait_for_version() {
        use std::thread;

        let index = std::sync::Arc::new(InvertedIndex::new());
        index.enable();

        let index_clone = index.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            let doc_ref = test_doc_ref("test");
            index_clone.index_document(&doc_ref, "hello", None);
        });

        // Wait for version to increment
        let result = index.wait_for_version(1, Duration::from_secs(1));
        handle.join().unwrap();

        assert!(result);
        assert!(index.version() >= 1);
    }

    #[test]
    fn test_wait_for_version_timeout() {
        let index = InvertedIndex::new();

        // Version is 0, waiting for 100 should timeout
        let result = index.wait_for_version(100, Duration::from_millis(10));
        assert!(!result);
    }

    #[test]
    fn test_posting_list() {
        let mut list = PostingList::new();
        assert!(list.is_empty());

        list.add(PostingEntry::new(42, 1, 10));

        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());

        let removed = list.remove_by_id(42);
        assert_eq!(removed, 1);
        assert!(list.is_empty());
    }

    #[test]
    fn test_resolve_doc_id() {
        let index = InvertedIndex::new();
        index.enable();

        let doc_ref = test_doc_ref("resolve_test");
        index.index_document(&doc_ref, "hello world", None);

        // The first doc should get doc_id 0
        let postings = index.lookup("hello").unwrap();
        let doc_id = postings.entries[0].doc_id;

        let resolved = index.resolve_doc_id(doc_id).unwrap();
        assert_eq!(resolved, doc_ref);
    }

    #[test]
    fn test_reindex_same_document() {
        let index = InvertedIndex::new();
        index.enable();

        let doc_ref = test_doc_ref("reindex");
        index.index_document(&doc_ref, "hello world", None);
        assert_eq!(index.total_docs(), 1);

        // Re-index same doc with different content (use pre-stemmed tokens)
        index.index_document(&doc_ref, "planet world extra", None);
        assert_eq!(index.total_docs(), 1);
        assert_eq!(index.doc_freq("hello"), 0); // old term gone
        assert_eq!(index.doc_freq("planet"), 1); // new term present
        assert_eq!(index.doc_freq("world"), 1); // shared term still 1
    }

    // ====================================================================
    // score_top_k tests
    // ====================================================================

    /// Helper: create a KV EntityRef with a specific branch_id
    fn kv_ref(branch_id: BranchId, key: &str) -> EntityRef {
        EntityRef::Kv {
            branch_id,
            key: key.to_string(),
        }
    }

    #[test]
    fn test_score_top_k_disabled_index() {
        let index = InvertedIndex::new();
        // Index is disabled by default
        let branch_id = BranchId::new();
        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);
        assert!(result.is_empty());
    }

    #[test]
    fn test_score_top_k_empty_query() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();
        let doc = kv_ref(branch_id, "doc1");
        index.index_document(&doc, "hello world", None);

        let result = index.score_top_k(&[], &branch_id, 10, 0.9, 0.4);
        assert!(result.is_empty());
    }

    #[test]
    fn test_score_top_k_k_zero() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();
        let doc = kv_ref(branch_id, "doc1");
        index.index_document(&doc, "hello world", None);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 0, 0.9, 0.4);
        assert!(result.is_empty());
    }

    #[test]
    fn test_score_top_k_no_matching_term() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();
        let doc = kv_ref(branch_id, "doc1");
        index.index_document(&doc, "hello world", None);

        // Query with a term that doesn't exist in the index
        let terms = vec!["nonexistent".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);
        assert!(result.is_empty());
    }

    #[test]
    fn test_score_top_k_basic_single_term() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        let doc1 = kv_ref(branch_id, "doc1");
        let doc2 = kv_ref(branch_id, "doc2");
        index.index_document(&doc1, "hello world", None);
        index.index_document(&doc2, "hello planet", None);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        // Both docs contain "hello", so both should appear
        assert_eq!(result.len(), 2);
        // All scores should be positive
        assert!(result[0].score > 0.0);
        assert!(result[1].score > 0.0);
        // Results should be sorted descending by score
        assert!(result[0].score >= result[1].score);
    }

    #[test]
    fn test_score_top_k_branch_filtering() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_a = BranchId::new();
        let branch_b = BranchId::new();

        let doc_a = kv_ref(branch_a, "doc_a");
        let doc_b = kv_ref(branch_b, "doc_b");
        index.index_document(&doc_a, "hello world", None);
        index.index_document(&doc_b, "hello planet", None);

        // Search branch_a: should only find doc_a
        let terms = vec!["hello".to_string()];
        let result_a = index.score_top_k(&terms, &branch_a, 10, 0.9, 0.4);
        assert_eq!(result_a.len(), 1);
        let resolved = index.resolve_doc_id(result_a[0].doc_id).unwrap();
        assert_eq!(resolved, doc_a);

        // Search branch_b: should only find doc_b
        let result_b = index.score_top_k(&terms, &branch_b, 10, 0.9, 0.4);
        assert_eq!(result_b.len(), 1);
        let resolved = index.resolve_doc_id(result_b[0].doc_id).unwrap();
        assert_eq!(resolved, doc_b);
    }

    #[test]
    fn test_score_top_k_respects_k_limit() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // Index 20 documents all containing "common"
        for i in 0..20 {
            let doc = kv_ref(branch_id, &format!("doc{}", i));
            index.index_document(&doc, &format!("common word{}", i), None);
        }

        let terms = vec!["common".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 5, 0.9, 0.4);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_score_top_k_multi_term_accumulation() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // doc1 matches both query terms, doc2 matches only one
        let doc1 = kv_ref(branch_id, "doc1");
        let doc2 = kv_ref(branch_id, "doc2");
        index.index_document(&doc1, "hello world", None);
        index.index_document(&doc2, "hello planet", None);

        let terms = vec!["hello".to_string(), "world".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        assert_eq!(result.len(), 2);
        // doc1 matches both terms, so it should score higher
        let doc1_id = index.doc_id_map.get(&doc1).unwrap();
        let doc1_score = result.iter().find(|r| r.doc_id == doc1_id).unwrap().score;
        let doc2_id = index.doc_id_map.get(&doc2).unwrap();
        let doc2_score = result.iter().find(|r| r.doc_id == doc2_id).unwrap().score;
        assert!(
            doc1_score > doc2_score,
            "doc1 ({}) matches both terms, should score higher than doc2 ({})",
            doc1_score,
            doc2_score
        );
    }

    #[test]
    fn test_score_top_k_idf_weighting() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // "common" appears in all 10 docs, "rare" appears in only 1
        for i in 0..10 {
            let doc = kv_ref(branch_id, &format!("doc{}", i));
            if i == 0 {
                index.index_document(&doc, "common rare", None);
            } else {
                index.index_document(&doc, "common filler", None);
            }
        }

        // Search for "rare" — only doc0 should match
        let rare_terms = vec!["rare".to_string()];
        let rare_result = index.score_top_k(&rare_terms, &branch_id, 10, 0.9, 0.4);
        assert_eq!(rare_result.len(), 1);

        // Search for "common" — all 10 docs match
        let common_terms = vec!["common".to_string()];
        let common_result = index.score_top_k(&common_terms, &branch_id, 10, 0.9, 0.4);
        assert_eq!(common_result.len(), 10);

        // The "rare" term score for doc0 should be higher than "common" term
        // score for doc0, because rare terms have higher IDF
        let doc0_id = index.doc_id_map.get(&kv_ref(branch_id, "doc0")).unwrap();
        let rare_score = rare_result.iter().find(|r| r.doc_id == doc0_id).unwrap().score;
        let common_score = common_result
            .iter()
            .find(|r| r.doc_id == doc0_id)
            .unwrap()
            .score;
        assert!(
            rare_score > common_score,
            "rare term score ({}) should be higher than common term score ({})",
            rare_score,
            common_score
        );
    }

    #[test]
    fn test_score_top_k_higher_tf_scores_higher() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // doc1 has "hello" once, doc2 has "hello" three times
        let doc1 = kv_ref(branch_id, "doc1");
        let doc2 = kv_ref(branch_id, "doc2");
        // Keep doc_len similar so TF is the main differentiator
        index.index_document(&doc1, "hello filler padding", None);
        index.index_document(&doc2, "hello hello hello", None);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        assert_eq!(result.len(), 2);
        let doc1_id = index.doc_id_map.get(&doc1).unwrap();
        let doc2_id = index.doc_id_map.get(&doc2).unwrap();
        let doc1_score = result.iter().find(|r| r.doc_id == doc1_id).unwrap().score;
        let doc2_score = result.iter().find(|r| r.doc_id == doc2_id).unwrap().score;
        assert!(
            doc2_score > doc1_score,
            "doc2 (tf=3, score={}) should beat doc1 (tf=1, score={})",
            doc2_score,
            doc1_score
        );
    }

    #[test]
    fn test_score_top_k_descending_order() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // Index several docs with varying relevance
        for i in 0..10 {
            let doc = kv_ref(branch_id, &format!("doc{}", i));
            // Repeat "target" i+1 times to create varying TF
            let text = format!("{} filler", "target ".repeat(i + 1));
            index.index_document(&doc, &text, None);
        }

        let terms = vec!["target".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        // Verify strictly descending order
        for window in result.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "Results not sorted: {} should be >= {}",
                window[0].score,
                window[1].score
            );
        }
    }

    #[test]
    fn test_score_top_k_scores_match_bm25_formula() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // Single doc, single term: verify exact BM25 calculation
        let doc = kv_ref(branch_id, "doc1");
        index.index_document(&doc, "hello hello world", None);

        let k1 = 1.2_f32;
        let b = 0.75_f32;
        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, k1, b);
        assert_eq!(result.len(), 1);

        // Manual BM25 calculation:
        // total_docs = 1, df("hello") = 1
        // idf = ln((1 - 1 + 0.5) / (1 + 0.5) + 1) = ln(1.333...) ≈ 0.2877
        let total_docs = 1.0_f32;
        let df = 1.0_f32;
        let expected_idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();

        // tf = 2 (hello appears twice), doc_len = 3, avg_doc_len = 3
        let tf = 2.0_f32;
        let dl = 3.0_f32;
        let avg_dl = 3.0_f32;
        let tf_comp = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avg_dl));
        let expected_score = expected_idf * tf_comp;

        assert!(
            (result[0].score - expected_score).abs() < 1e-5,
            "Score {} should match expected BM25 {}",
            result[0].score,
            expected_score
        );
    }

    #[test]
    fn test_score_top_k_nonexistent_branch() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_a = BranchId::new();
        let branch_nonexistent = BranchId::new();

        let doc = kv_ref(branch_a, "doc1");
        index.index_document(&doc, "hello world", None);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_nonexistent, 10, 0.9, 0.4);
        assert!(result.is_empty(), "No docs should match a non-existent branch");
    }

    #[test]
    fn test_score_top_k_k_larger_than_matches() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        let doc = kv_ref(branch_id, "doc1");
        index.index_document(&doc, "hello world", None);

        // Request k=100 but only 1 doc matches
        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 100, 0.9, 0.4);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_score_top_k_after_document_removal() {
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        let doc1 = kv_ref(branch_id, "doc1");
        let doc2 = kv_ref(branch_id, "doc2");
        index.index_document(&doc1, "hello world", None);
        index.index_document(&doc2, "hello planet", None);

        // Remove doc1
        index.remove_document(&doc1);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        // Only doc2 should remain
        assert_eq!(result.len(), 1);
        let resolved = index.resolve_doc_id(result[0].doc_id).unwrap();
        assert_eq!(resolved, doc2);
    }

    #[test]
    fn test_score_top_k_with_non_kv_entity() {
        // Ensure non-KV entities are handled correctly (filtered by branch_id)
        let index = InvertedIndex::new();
        index.enable();
        let branch_id = BranchId::new();

        // Index a KV entity and an Event entity on the same branch
        let kv_doc = EntityRef::Kv {
            branch_id,
            key: "doc1".to_string(),
        };
        let event_doc = EntityRef::Event {
            branch_id,
            sequence: 42,
        };
        index.index_document(&kv_doc, "hello world", None);
        index.index_document(&event_doc, "hello planet", None);

        let terms = vec!["hello".to_string()];
        let result = index.score_top_k(&terms, &branch_id, 10, 0.9, 0.4);

        // Both should match since we filter by branch_id, not entity type
        assert_eq!(result.len(), 2);
    }
}
