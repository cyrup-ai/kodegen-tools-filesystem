//! Waiting and sorting logic for search results

use super::super::super::types::{SearchSession, SortBy, SortDirection};
use kodegen_mcp_tool::error::McpError;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Wait for first result or timeout (streaming mode)
///
/// Returns after receiving first result notification or 40ms timeout
pub async fn wait_for_first_result(first_result_rx: &mut watch::Receiver<bool>) {
    use tokio::time::timeout;
    let _ = timeout(Duration::from_millis(40), first_result_rx.changed()).await;
}

/// Wait for search completion (sorting mode)
///
/// Waits up to 30 seconds for the search to complete, keeping session alive
///
/// # Errors
/// Returns error if session is lost during wait
pub async fn wait_for_completion(
    sessions: &tokio::sync::RwLock<HashMap<String, SearchSession>>,
    session_id: &str,
) -> Result<(), McpError> {
    let timeout_duration = Duration::from_secs(30); // Max 30s wait for sorting
    let wait_start = Instant::now();

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let sessions_guard = sessions.read().await;
        let session = sessions_guard
            .get(session_id)
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session lost during wait")))?;

        // Keep session alive during wait - prevents cleanup while legitimately waiting
        let elapsed_micros = session.start_time.elapsed().as_micros() as u64;
        session
            .last_read_time_atomic
            .store(elapsed_micros, Ordering::Relaxed);

        let is_complete = session.is_complete.load(Ordering::Acquire);
        let is_error = *session.is_error.read().await;

        if is_complete || is_error || wait_start.elapsed() >= timeout_duration {
            break;
        }
    }

    Ok(())
}

/// Apply sorting to search results
///
/// # Errors
/// Returns error if session is lost during sorting
pub async fn apply_sorting(
    sessions: &tokio::sync::RwLock<HashMap<String, SearchSession>>,
    session_id: &str,
    sort_by: Option<SortBy>,
    sort_direction: Option<SortDirection>,
) -> Result<(), McpError> {
    let sessions_guard = sessions.read().await;
    let session = sessions_guard
        .get(session_id)
        .ok_or_else(|| McpError::Other(anyhow::anyhow!("Session lost after search")))?;

    let mut results = session.results.write().await;

    if let Some(sort_criterion) = sort_by {
        use crate::search::sorting::{
            SortBy as SortCriterion, SortDirection as SortDir, sort_results,
        };

        let sort_by_criterion = match sort_criterion {
            SortBy::Path => SortCriterion::Path,
            SortBy::Modified => SortCriterion::Modified,
            SortBy::Accessed => SortCriterion::Accessed,
            SortBy::Created => SortCriterion::Created,
        };

        let sort_dir = match sort_direction.unwrap_or(SortDirection::Ascending) {
            SortDirection::Ascending => SortDir::Ascending,
            SortDirection::Descending => SortDir::Descending,
        };

        sort_results(&mut results, sort_by_criterion, sort_dir);
    }

    Ok(())
}
