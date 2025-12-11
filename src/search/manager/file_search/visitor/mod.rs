//! File search visitor implementation

pub(super) mod buffering;
pub(super) mod errors;
pub(super) mod matching;
mod visit_impl;

use crate::search::types::{CaseMode, SearchError, SearchResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use tokio::sync::RwLock;

/// Compiled pattern for matching filenames
#[derive(Clone)]
pub(super) enum CompiledPattern {
    /// Compiled regex pattern
    Regex(regex::Regex),
    /// Compiled glob pattern  
    Glob(globset::GlobMatcher),
    /// Plain substring (uses pattern/pattern_lower fields)
    Substring,
}

/// Parallel visitor for file search
pub(super) struct FileSearchVisitor {
    pub(super) compiled_pattern: CompiledPattern,
    pub(super) pattern: String,
    pub(super) pattern_lower: String,
    pub(super) case_mode: CaseMode,
    pub(super) is_pattern_lowercase: bool,
    pub(super) word_boundary: bool,
    pub(super) max_results: usize,
    pub(super) early_termination: bool,
    pub(super) early_term_triggered: Arc<AtomicBool>,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    /// Thread-local buffer for batching results
    pub(super) buffer: Vec<SearchResult>,
}

impl FileSearchVisitor {
    /// Check if this is an exact match (not a partial/wildcard match)
    pub(super) fn is_exact_match(&self, file_name: &str) -> bool {
        matching::is_exact_match(
            &self.compiled_pattern,
            &self.pattern,
            self.case_mode,
            self.is_pattern_lowercase,
            self.word_boundary,
            file_name,
        )
    }

    /// Track a directory traversal error
    pub(super) fn track_error(&self, error: &ignore::Error) {
        errors::track_error(error, &self.error_count, &self.errors);
    }

    /// Flush buffered results to shared storage
    pub(super) fn flush_buffer(&mut self) {
        buffering::flush_buffer(&mut self.buffer, &self.results);
    }

    /// Add result to buffer, flush if full
    pub(super) fn add_result(&mut self, result: SearchResult) {
        buffering::add_result(&mut self.buffer, result, &self.results);
    }
}

impl Drop for FileSearchVisitor {
    fn drop(&mut self) {
        // Flush any remaining buffered results
        // This is CRITICAL - prevents losing the last batch of results
        self.flush_buffer();
    }
}
