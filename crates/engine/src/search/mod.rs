//! Search module for keyword and retrieval operations
//!
//! This module contains:
//! - `types`: Core search types (SearchRequest, SearchResponse, SearchHit, etc.)
//! - `searchable`: Searchable trait and scoring infrastructure
//! - `index`: Optional inverted index for fast keyword search
//! - `tokenizer`: Basic text tokenization

mod index;
mod searchable;
pub mod stemmer;
pub mod tokenizer;
mod types;

pub use index::{InvertedIndex, PostingEntry, PostingList, ScoredDocId};
pub use searchable::{
    build_search_response, build_search_response_with_index, build_search_response_with_scorer,
    truncate_text, BM25LiteScorer, Scorer, ScorerContext, SearchCandidate, SearchDoc, Searchable,
    SimpleScorer,
};
pub use tokenizer::{tokenize, tokenize_unique};
pub use types::{
    EntityRef, PrimitiveType, SearchBudget, SearchHit, SearchMode, SearchRequest, SearchResponse,
    SearchStats,
};
