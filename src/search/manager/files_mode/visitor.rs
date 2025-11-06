//!
//! Parallel visitor for files mode

use super::super::config::{
    LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD,
    MAX_DETAILED_ERRORS, RESULT_BUFFER_SIZE,
};
use super::super::super::types::{SearchError, SearchResult, SearchResultType};

use ignore::{DirEntry, ParallelVisitor};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor for files mode
pub(super) struct FilesListerVisitor {
    pub(super) max_results: usize,
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
    pub(super) last_update_time: Instant,
    /// Number of matches since last update
    pub(super) matches_since_update: usize,
    pub(super) start_time: Instant,
}

impl FilesListerVisitor {
    /// Update `last_read_time` if enough time has passed or enough matches accumulated
    fn maybe_update_last_read_time(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update_time);

        if elapsed.as_millis() >= u128::from(LAST_READ_UPDATE_INTERVAL_MS)
            || self.matches_since_update >= LAST_READ_UPDATE_MATCH_THRESHOLD
        {
            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
            self.last_read_time_atomic
                .store(elapsed_micros, Ordering::Relaxed);
            self.last_update_time = now;
            self.matches_since_update = 0;
        }
    }

    /// Flush buffered results to shared results
    pub(super) fn flush_buffer(&mut self) {
        if !self.buffer.is_empty() {
            // Check if this is the first batch of results
            let was_empty = self.results.blocking_read().is_empty();

            let mut results = self.results.blocking_write();
            results.extend(self.buffer.drain(..));
            drop(results); // Release lock before calling maybe_update_last_read_time

            // Signal first result if this was the first batch
            if was_empty {
                let _ = self.first_result_tx.send(true);
            }

            self.maybe_update_last_read_time();
        }
    }

    /// Add a file to the buffer
    fn add_file(&mut self, entry: &DirEntry) {
        let entry_metadata = entry.metadata().ok();
        let modified = entry_metadata.as_ref().and_then(|m| m.modified().ok());
        let accessed = entry_metadata.as_ref().and_then(|m| m.accessed().ok());
        let created = entry_metadata.as_ref().and_then(|m| m.created().ok());

        let result = SearchResult {
            file: entry.path().display().to_string(),
            line: None,
            r#match: None,
            r#type: SearchResultType::FileList,
            is_context: false,
            is_binary: None,
            binary_suppressed: None,
            modified,
            accessed,
            created,
        };

        self.buffer.push(result);
        self.matches_since_update += 1;

        // Flush when buffer is full
        if self.buffer.len() >= RESULT_BUFFER_SIZE {
            self.flush_buffer();
        }
    }
}

impl Drop for FilesListerVisitor {
    fn drop(&mut self) {
        // Flush any remaining buffered results
        // This is CRITICAL - prevents losing the last batch of results
        self.flush_buffer();
        // Ensure final last_read_time update
        let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
        self.last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);
    }
}

impl ParallelVisitor for FilesListerVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Check cancellation
        if *self.cancellation_rx.borrow() {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        // Check max results
        let current_total = self.total_matches.load(Ordering::Relaxed);
        if current_total >= self.max_results {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        match entry {
            Ok(entry) => {
                // Only process files, not directories
                if let Some(file_type) = entry.file_type()
                    && file_type.is_file()
                {
                    self.add_file(&entry);
                    self.total_matches.fetch_add(1, Ordering::Relaxed);
                }
                ignore::WalkState::Continue
            }
            Err(err) => {
                // Record error
                self.error_count.fetch_add(1, Ordering::Relaxed);

                let mut errors = self.errors.blocking_write();
                if errors.len() < MAX_DETAILED_ERRORS {
                    errors.push(SearchError {
                        path: "unknown".to_string(),
                        message: err.to_string(),
                        error_type: "walk_error".to_string(),
                    });
                }
                drop(errors);

                ignore::WalkState::Continue
            }
        }
    }
}
