//! Search manager for coordinating streaming search operations
//!
//! This module provides the main `SearchManager` API for starting, managing,
//! and retrieving results from background search tasks.

mod cleanup;
mod operations;
mod session;
mod spawn;
mod waiting;

use super::super::types::{
    GetMoreSearchResultsResponse, SearchSession, SearchSessionInfo, SearchSessionOptions,
    StartSearchResponse,
};
use kodegen_mcp_tool::error::McpError;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{RwLock, watch};

/// Manager for streaming search operations
#[derive(Clone)]
pub struct SearchManager {
    sessions: Arc<RwLock<HashMap<String, SearchSession>>>,
    config_manager: kodegen_config_manager::ConfigManager,
}

impl SearchManager {
    /// Create a new search manager
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config_manager,
        }
    }

    /// Start a new streaming search session
    ///
    /// # Errors
    /// Returns error if path validation fails or search cannot be started
    pub async fn start_search(
        &self,
        mut options: SearchSessionOptions,
    ) -> Result<StartSearchResponse, McpError> {
        // Enforce result limits before starting search
        let effective_max_results = session::enforce_result_limits(&mut options);

        log::debug!("Starting search with effective max_results: {effective_max_results}");

        // Validate path and generate session ID
        let (validated_path, search_id) =
            session::validate_and_generate_id(&options, &self.config_manager).await?;

        // Create channels for search cancellation and first-result notification
        let (cancellation_tx, cancellation_rx) = watch::channel(false);
        let (first_result_tx, mut first_result_rx_for_wait) = watch::channel(false);

        // Build session object
        let search_session = session::build_session(
            search_id.clone(),
            &options,
            effective_max_results,
            cancellation_tx,
            first_result_tx,
        );

        // Insert session (collision check not needed for UUID v4)
        {
            let mut sessions = self.sessions.write().await;

            // Defensive check (debug builds only) - if this fails, RNG is broken
            debug_assert!(
                !sessions.contains_key(&search_id),
                "IMPOSSIBLE: UUID v4 collision detected! RNG may be compromised: {}",
                search_id
            );

            sessions.insert(search_id.clone(), search_session);
        }
        // Lock automatically dropped here

        // Capture sort options before moving options into spawn_search_task
        let sort_by = options.sort_by;
        let sort_direction = options.sort_direction;

        // Spawn background search task
        spawn::spawn_search_task(
            search_id.clone(),
            options,
            validated_path,
            cancellation_rx,
            Arc::clone(&self.sessions),
        );

        // If sorting is enabled, wait for search to complete before returning
        if sort_by.is_some() {
            // Wait for search to complete or timeout
            waiting::wait_for_completion(&self.sessions, &search_id).await?;

            // Apply sorting to results
            waiting::apply_sorting(&self.sessions, &search_id, sort_by, sort_direction).await?;
        } else {
            // No sorting: wait up to 5s for first result (makes fast searches trivial for agents)
            waiting::wait_for_first_result(&mut first_result_rx_for_wait).await;
        }

        // Return initial state
        let sessions = self.sessions.read().await;
        let session = sessions.get(&search_id).ok_or_else(|| {
            McpError::Other(anyhow::anyhow!("Session lost during initialization"))
        })?;

        // Read status fields
        let is_complete = session.is_complete.load(Ordering::Acquire);
        let is_error = *session.is_error.read().await;

        // Lock coupling: Hold results lock while reading total_matches and taking initial slice
        // This ensures the returned results and total_matches are consistent snapshots
        let (initial_results, total_matches) = {
            let results = session.results.read().await;
            let total_matches = session.total_matches.load(Ordering::SeqCst);
            let initial_results = results.iter().take(10).cloned().collect();
            (initial_results, total_matches)
        };

        // Check if results were limited
        let results_limited = if total_matches >= effective_max_results {
            Some(true)
        } else {
            None
        };

        Ok(StartSearchResponse {
            search_id: search_id.clone(),
            is_complete,
            is_error,
            results: initial_results,
            total_results: total_matches,
            runtime_ms: u64::try_from(session.start_time.elapsed().as_millis()).unwrap_or(u64::MAX),
            error_count: session.error_count.load(Ordering::SeqCst),
            max_results: effective_max_results,
            results_limited,
        })
    }

    /// Terminate a search session by sending cancellation signal
    ///
    /// Sends cancellation signal to the blocking task, causing it to exit cleanly at next loop iteration.
    ///
    /// # Errors
    /// Returns error if session cannot be accessed (should not occur in practice)
    pub async fn terminate_search(&self, search_id: &str) -> Result<bool, McpError> {
        operations::terminate_search(&self.sessions, search_id).await
    }

    /// Get paginated results from an active search session
    ///
    /// # Errors
    /// Returns error if session not found
    pub async fn get_results(
        &self,
        search_id: &str,
        offset: i64,
        length: usize,
    ) -> Result<GetMoreSearchResultsResponse, McpError> {
        operations::get_results(&self.sessions, search_id, offset, length).await
    }

    /// List all active search sessions
    pub async fn list_active_sessions(&self) -> Vec<SearchSessionInfo> {
        operations::list_active_sessions(&self.sessions).await
    }

    /// Clean up old completed sessions with differentiated retention.
    ///
    /// Removes sessions based on completion status:
    /// - Completed searches: 30 seconds retention
    /// - Active searches: 5 minutes retention
    ///
    /// Recently-read sessions are preserved regardless of completion status.
    pub async fn cleanup_sessions(&self) {
        cleanup::cleanup_sessions(&self.sessions).await;
    }

    /// Start background cleanup task (call once on manager creation)
    ///
    /// Runs cleanup every minute with differentiated retention:
    /// - Active searches: 5 minutes retention
    /// - Completed searches: 30 seconds retention
    pub fn start_cleanup_task(self: Arc<Self>) {
        cleanup::start_cleanup_task(Arc::clone(&self.sessions));
    }
}
