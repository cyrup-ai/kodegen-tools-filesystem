//! File search visitor implementation

pub(super) mod buffering;
pub(super) mod errors;
pub(super) mod matching;
mod visit_impl;

use crate::search::types::{CaseMode, SearchError, SearchResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor for file search
pub(super) struct FileSearchVisitor {
    pub(super) glob_pattern: Option<globset::GlobMatcher>,
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
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    /// Thread-local buffer for batching results
    pub(super) buffer: Vec<SearchResult>,
    /// Last time we updated the shared `last_read_time`
    /// Used in `maybe_update_last_read_time()` to throttle lock acquisitions
    pub(super) last_update_time: Instant,
    pub(super) start_time: Instant,
    /// Number of matches since last update
    /// Used in `maybe_update_last_read_time()` to throttle lock acquisitions
    pub(super) matches_since_update: usize,
}

impl FileSearchVisitor {
    /// Check if this is an exact match (not a partial/wildcard match)
    pub(super) fn is_exact_match(&self, file_name: &str) -> bool {
        matching::is_exact_match(
            &self.glob_pattern,
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
        buffering::flush_buffer(
            &mut self.buffer,
            &self.results,
            &self.first_result_tx,
            &self.last_read_time_atomic,
            &self.start_time,
        );
    }

    /// Add result to buffer, flush if full
    pub(super) fn add_result(&mut self, result: SearchResult) {
        buffering::add_result(
            &mut self.buffer,
            result,
            &self.results,
            &self.first_result_tx,
            &self.last_read_time_atomic,
            &self.start_time,
        );
    }

    /// Update `last_read_time` if throttle threshold exceeded
    pub(super) fn maybe_update_last_read_time(&mut self) {
        buffering::maybe_update_last_read_time(
            &mut self.matches_since_update,
            &mut self.last_update_time,
            &self.last_read_time_atomic,
            &self.start_time,
        );
    }

    /// Force update `last_read_time` (used in Drop)
    pub(super) fn force_update_last_read_time(&mut self) {
        buffering::force_update_last_read_time(
            &mut self.last_update_time,
            &mut self.matches_since_update,
            &self.last_read_time_atomic,
            &self.start_time,
        );
    }
}

impl Drop for FileSearchVisitor {
    fn drop(&mut self) {
        // Flush any remaining buffered results
        // This is CRITICAL - prevents losing the last batch of results
        self.flush_buffer();
        // Ensure final last_read_time update
        self.force_update_last_read_time();
    }
}
