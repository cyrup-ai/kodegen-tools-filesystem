//! Error tracking for file search

use crate::search::manager::config::MAX_DETAILED_ERRORS;
use crate::search::types::SearchError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Track a directory traversal error
pub(super) fn track_error(
    error: &ignore::Error,
    error_count: &Arc<AtomicUsize>,
    errors: &Arc<RwLock<Vec<SearchError>>>,
) {
    error_count.fetch_add(1, Ordering::SeqCst);

    log::debug!("File search error: {error}");

    // Check if we should store BEFORE allocating
    let should_store = {
        let errors = errors.blocking_read();
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

        let mut errors = errors.blocking_write();
        // Double-check in case another thread added while we were allocating
        if errors.len() < MAX_DETAILED_ERRORS {
            errors.push(SearchError {
                path: path_str,
                message: error_str,
                error_type: categorize_error(error),
            });
        }
    }
}

/// Categorize an error by type
fn categorize_error(error: &ignore::Error) -> String {
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
