//! Core visitor implementation for content search

use super::super::super::types::{SearchError, ReturnMode, SearchResult};
use super::super::config::{MAX_DETAILED_ERRORS, RESULT_BUFFER_SIZE};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Parallel visitor for content search
pub(in super::super) struct ContentSearchVisitor {
    pub(super) worker: super::super::super::rg::search::SearchWorker<Vec<u8>>,
    pub(super) haystack_builder: super::super::super::rg::haystack::HaystackBuilder,
    pub(super) max_results: Option<usize>,
    pub(super) return_only: ReturnMode,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,
    pub(super) file_counts: Arc<RwLock<HashMap<String, super::super::super::types::FileCountData>>>,
    /// Thread-local buffer for batching results before flushing to shared storage
    pub(super) buffer: Vec<SearchResult>,
}

impl ContentSearchVisitor {
    /// Track a directory traversal error
    pub(super) fn track_error(&self, error: &ignore::Error) {
        // Increment atomic counter (lock-free)
        self.error_count.fetch_add(1, Ordering::SeqCst);

        // Log at debug level
        log::debug!("Search error: {error}");

        // Check if we should store BEFORE allocating
        let should_store = {
            let errors = self.errors.blocking_read();
            errors.len() < MAX_DETAILED_ERRORS
        };

        if should_store {
            // Only allocate if we're going to use it
            let error_str = error.to_string();
            let path_str = error_str
                .split(':')
                .next()
                .unwrap_or("<unknown>")
                .to_string();

            let mut errors = self.errors.blocking_write();
            // Double-check in case another thread added while we were allocating
            if errors.len() < MAX_DETAILED_ERRORS {
                errors.push(SearchError {
                    path: path_str,
                    message: error_str,
                    error_type: Self::categorize_ignore_error(error),
                });
            }
        }
    }

    /// Categorize `ignore::Error` for user-friendly display
    fn categorize_ignore_error(error: &ignore::Error) -> String {
        let err_str = error.to_string().to_lowercase();
        if err_str.contains("permission denied") {
            "permission_denied".to_string()
        } else if err_str.contains("broken pipe") || err_str.contains("i/o error") {
            "io_error".to_string()
        } else if err_str.contains("invalid") {
            "invalid_path".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Flush buffered results to shared storage
    ///
    /// Acquires write lock ONCE for entire buffer batch.
    /// This is the core optimization: batch writes reduce lock contention.
    pub(super) fn flush_buffer(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        // Single lock acquisition for entire buffer
        {
            let mut results_guard = self.results.blocking_write();
            results_guard.extend(self.buffer.drain(..));
        }
    }

    /// Add result to thread-local buffer, flush if full
    ///
    /// This replaces direct `results.push()` calls to avoid per-match locking.
    pub(super) fn add_result(&mut self, result: SearchResult) {
        self.buffer.push(result);

        // Flush when buffer reaches capacity
        if self.buffer.len() >= RESULT_BUFFER_SIZE {
            self.flush_buffer();
        }
    }
}

impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        // CRITICAL: Flush any remaining buffered results
        // Without this, the last batch of results would be lost!
        self.flush_buffer();
    }
}
