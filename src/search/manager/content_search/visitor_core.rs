//! Core visitor implementation for content search

use super::super::super::types::{SearchError, ReturnMode, SearchResult};
use super::super::config::{
    LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD, MAX_DETAILED_ERRORS,
    RESULT_BUFFER_SIZE,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor for content search
pub(in super::super) struct ContentSearchVisitor {
    pub(super) worker: super::super::super::rg::search::SearchWorker<Vec<u8>>,
    pub(super) haystack_builder: super::super::super::rg::haystack::HaystackBuilder,
    pub(super) max_results: Option<usize>,
    pub(super) return_only: ReturnMode,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,
    pub(super) file_counts: Arc<RwLock<HashMap<String, super::super::super::types::FileCountData>>>,
    pub(super) start_time: Instant,
    /// Thread-local buffer for batching results before flushing to shared storage
    pub(super) buffer: Vec<SearchResult>,
    /// Last time we updated the shared `last_read_time` (for throttling)
    pub(super) last_update_time: Instant,
    /// Number of matches since last timestamp update (for throttling)
    pub(super) matches_since_update: usize,
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

        // Check if this is the first batch of results
        let was_empty = self.results.blocking_read().is_empty();

        // Single lock acquisition for entire buffer
        {
            let mut results_guard = self.results.blocking_write();
            results_guard.extend(self.buffer.drain(..));
        }

        // Signal first result if this was the first batch
        if was_empty {
            let _ = self.first_result_tx.send(true);
        }

        // Update last read time once per flush
        {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
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

    /// Update `last_read_time` if throttle threshold exceeded
    ///
    /// Prevents excessive atomic stores by updating only every N matches or T milliseconds.
    pub(super) fn maybe_update_last_read_time(&mut self) {
        self.matches_since_update += 1;

        let now = Instant::now();
        let time_since_update = now.duration_since(self.last_update_time);

        // Update if time threshold OR match count threshold exceeded
        let should_update = time_since_update.as_millis() as u64 >= LAST_READ_UPDATE_INTERVAL_MS
            || self.matches_since_update >= LAST_READ_UPDATE_MATCH_THRESHOLD;

        if should_update {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
            self.last_update_time = now;
            self.matches_since_update = 0;
        }
    }

    /// Force update `last_read_time` (used in Drop)
    pub(super) fn force_update_last_read_time(&mut self) {
        let now = Instant::now();
        let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
        self.last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);
        self.last_update_time = now;
        self.matches_since_update = 0;
    }
}

impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        // CRITICAL: Flush any remaining buffered results
        // Without this, the last batch of results would be lost!
        self.flush_buffer();

        // Ensure final last_read_time update
        self.force_update_last_read_time();
    }
}
