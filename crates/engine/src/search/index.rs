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
        PostingEntry { doc_id, tf, doc_len }
    }
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
}
