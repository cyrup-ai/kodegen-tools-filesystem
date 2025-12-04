//!
//! Parallel visitor for files mode

use super::super::config::{MAX_DETAILED_ERRORS, RESULT_BUFFER_SIZE};
use super::super::super::types::{SearchError, SearchResult, SearchResultType};

use ignore::{DirEntry, ParallelVisitor};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Parallel visitor for files mode
pub(super) struct FilesListerVisitor {
    pub(super) max_results: usize,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    /// Thread-local buffer for batching results
    pub(super) buffer: Vec<SearchResult>,
}

impl FilesListerVisitor {
    /// Flush buffered results to shared results
    pub(super) fn flush_buffer(&mut self) {
        if !self.buffer.is_empty() {
            let mut results = self.results.blocking_write();
            results.extend(self.buffer.drain(..));
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
    }
}

impl ParallelVisitor for FilesListerVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Check max results
        let current_total = self.total_matches.load(Ordering::Relaxed);
        if current_total >= self.max_results {
            self.flush_buffer();
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
                    self.total_files.fetch_add(1, Ordering::Relaxed);
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
