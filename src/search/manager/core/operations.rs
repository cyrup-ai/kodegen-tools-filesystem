//! Search session operations (terminate, get_more, list)

use super::super::super::types::{
    GetMoreSearchResultsResponse, SearchSession, SearchSessionInfo, SearchType,
};
use kodegen_mcp_tool::error::McpError;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;

/// Terminate a search session by sending cancellation signal
///
/// Sends cancellation signal to the blocking task, causing it to exit cleanly at next loop iteration.
///
/// # Errors
/// Returns error if session cannot be accessed (should not occur in practice)
pub async fn terminate_search(
    sessions: &RwLock<HashMap<String, SearchSession>>,
    session_id: &str,
) -> Result<bool, McpError> {
    let sessions_guard = sessions.read().await;

    let Some(session) = sessions_guard.get(session_id) else {
        return Ok(false); // Session not found
    };

    if session.is_complete.load(Ordering::Acquire) {
        return Ok(false); // Already complete, nothing to cancel
    }

    // Send cancellation signal - This actually stops the task!
    if let Ok(()) = session.cancellation_tx.send(true) {
        log::info!("Sent cancellation signal to search session: {session_id}");
        Ok(true) // Signal sent successfully
    } else {
        // Channel closed means receiver dropped (task already finished)
        log::debug!("Search session {session_id} already finished");
        Ok(false)
    }
}

/// Get paginated results from an active search session
///
/// # Errors
/// Returns error if session not found
pub async fn get_results(
    sessions: &RwLock<HashMap<String, SearchSession>>,
    session_id: &str,
    offset: i64,
    length: usize,
) -> Result<GetMoreSearchResultsResponse, McpError> {
    let sessions_guard = sessions.read().await;
    let session = sessions_guard.get(session_id).ok_or_else(|| {
        McpError::InvalidArguments(format!("Search session {session_id} not found"))
    })?;

    // Read status fields first (no lock coupling needed for these)
    let is_complete = session.is_complete.load(Ordering::Acquire);
    let is_error = *session.is_error.read().await;
    let error = session.error.read().await.clone();
    let was_incomplete = *session.was_incomplete.read().await;

    // Lock coupling: Hold results lock while reading total_matches and performing slicing
    // This ensures total_results and total_matches are consistent snapshots from the same instant
    let (sliced_results, total_results, total_matches, has_more) = {
        let results = session.results.read().await; // Acquire read lock

        // Capture both counts WHILE holding lock
        let total_results = results.len();
        let total_matches = session.total_matches.load(Ordering::SeqCst);

        // Perform slicing WHILE holding lock (ensures consistency)
        let (sliced, has_more) = if offset < 0 {
            // Negative offset: tail slicing - O(tail_count)
            let tail_count = usize::try_from(-offset).unwrap_or(0);

            // Direct slice from end - no iterator overhead
            let start = results.len().saturating_sub(tail_count);
            let tail_results = results[start..].to_vec();

            (tail_results, false)
        } else {
            // Positive offset: range slicing - O(length)
            let start = usize::try_from(offset).unwrap_or(0);

            if start >= results.len() {
                // Start past end - return empty
                (Vec::new(), !is_complete)
            } else {
                // Direct slice - no iterator overhead
                let end = start.saturating_add(length).min(results.len());
                let sliced = results[start..end].to_vec();
                let has_more = end < results.len() || !is_complete;

                (sliced, has_more)
            }
        };

        (sliced, total_results, total_matches, has_more)
    }; // Read lock released here - all values are consistent

    let elapsed_micros = session.start_time.elapsed().as_micros() as u64;
    session
        .last_read_time_atomic
        .store(elapsed_micros, Ordering::Relaxed);

    let error_count = session.error_count.load(Ordering::SeqCst);
    let errors = session.errors.read().await.clone();

    // Check if results were limited by max_results
    let results_limited = if total_matches >= session.max_results {
        Some(true)
    } else {
        None
    };

    Ok(GetMoreSearchResultsResponse {
        session_id: session_id.to_string(),
        results: sliced_results.clone(),
        returned_count: sliced_results.len(),
        total_results,
        total_matches,
        is_complete,
        is_error: is_error && error.is_some(),
        error,
        has_more_results: has_more,
        runtime_ms: u64::try_from(session.start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
        was_incomplete: if was_incomplete { Some(true) } else { None },
        error_count,
        errors,
        results_limited,
    })
}

/// List all active search sessions
pub async fn list_active_sessions(
    sessions: &RwLock<HashMap<String, SearchSession>>,
) -> Vec<SearchSessionInfo> {
    let sessions_guard = sessions.read().await;
    let mut result = Vec::new();

    for session in sessions_guard.values() {
        let is_complete = session.is_complete.load(Ordering::Acquire);
        let is_error = *session.is_error.read().await;
        let total_results = session.results.read().await.len();

        result.push(SearchSessionInfo {
            id: session.id.clone(),
            search_type: match session.search_type {
                SearchType::Files => "files".to_string(),
                SearchType::Content => "content".to_string(),
            },
            pattern: session.pattern.clone(),
            is_complete,
            is_error,
            runtime_ms: u64::try_from(session.start_time.elapsed().as_millis())
                .unwrap_or(u64::MAX),
            total_results,
            timeout_ms: session.timeout_ms,
            was_incomplete: if *session.was_incomplete.read().await {
                Some(true)
            } else {
                None
            },
        });
    }

    result
}
