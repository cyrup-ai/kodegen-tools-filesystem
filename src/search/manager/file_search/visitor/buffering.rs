//! Buffer management for file search results

use crate::search::manager::config::RESULT_BUFFER_SIZE;
use crate::search::types::SearchResult;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Flush buffered results to shared storage
pub(super) fn flush_buffer(
    buffer: &mut Vec<SearchResult>,
    results: &Arc<RwLock<Vec<SearchResult>>>,
) {
    if buffer.is_empty() {
        return;
    }

    // Single lock acquisition for entire buffer
    {
        let mut results_guard = results.blocking_write();
        results_guard.extend(buffer.drain(..));
    }
}

/// Add result to buffer, flush if full
pub(super) fn add_result(
    buffer: &mut Vec<SearchResult>,
    result: SearchResult,
    results: &Arc<RwLock<Vec<SearchResult>>>,
) {
    buffer.push(result);

    if buffer.len() >= RESULT_BUFFER_SIZE {
        flush_buffer(buffer, results);
    }
}
