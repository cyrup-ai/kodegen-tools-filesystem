//! Error visitor for handling initialization failures

use super::super::super::types::SearchError;
use ignore::{DirEntry, ParallelVisitor};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Fallback visitor used when per-thread initialization fails.
/// Records the error and immediately terminates the search gracefully.
pub(in super::super) struct ErrorVisitor {
    pub(super) error_message: String,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
}

impl ParallelVisitor for ErrorVisitor {
    fn visit(&mut self, _entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Only the first thread to encounter this error records it
        // This prevents duplicate error messages from multiple threads
        if self.error_count.fetch_add(1, Ordering::SeqCst) == 0 {
            let mut errors = self.errors.blocking_write();
            errors.push(SearchError {
                path: "<initialization>".to_string(),
                message: self.error_message.clone(),
                error_type: "initialization_error".to_string(),
            });
            *self.was_incomplete.blocking_write() = true;
        }

        // Immediately quit to prevent further thread spawning
        ignore::WalkState::Quit
    }
}
