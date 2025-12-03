//! Search session registry with connection isolation
//!
//! Manages multiple search instances keyed by (connection_id, search_id).
//! Each connection has its own isolated set of search instances.

use anyhow::{anyhow, Result};
use kodegen_mcp_schema::filesystem::{FsSearchOutput, FsSearchSnapshot};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::session::SearchSession;

type SearchMap = HashMap<(String, u32), Arc<SearchSession>>;

/// Registry for managing multiple search instances
#[derive(Clone)]
pub struct SearchRegistry {
    searches: Arc<Mutex<SearchMap>>,
}

impl SearchRegistry {
    /// Create a new search registry
    pub fn new() -> Self {
        Self {
            searches: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Find or create a search instance
    pub async fn find_or_create_search(
        &self,
        connection_id: &str,
        search_id: u32,
    ) -> Result<Arc<SearchSession>> {
        let key = (connection_id.to_string(), search_id);
        let mut searches = self.searches.lock().await;

        if let Some(session) = searches.get(&key) {
            return Ok(session.clone());
        }

        let session = Arc::new(SearchSession::new(search_id));
        searches.insert(key, session.clone());
        Ok(session)
    }

    /// List all active searches for a connection with their current states
    pub async fn list_all_searches(&self, connection_id: &str) -> Result<FsSearchOutput> {
        let start = std::time::Instant::now();
        let searches = self.searches.lock().await;
        let mut snapshots = Vec::new();

        for ((conn_id, search_id), session) in searches.iter() {
            if conn_id == connection_id {
                let state = session.get_snapshot().await;
                snapshots.push(FsSearchSnapshot {
                    search: *search_id,
                    pattern: if state.pattern.is_empty() { None } else { Some(state.pattern) },
                    path: if state.path.is_empty() { None } else { Some(state.path) },
                    match_count: state.match_count,
                    files_searched: state.files_searched,
                    completed: state.completed,
                    duration_ms: Some(state.duration_ms),
                });
            }
        }

        // Sort by search ID
        snapshots.sort_by_key(|s| s.search);

        let output = serde_json::to_string_pretty(&snapshots)?;

        Ok(FsSearchOutput {
            search: None, // None indicates LIST response with multiple searches
            output,
            pattern: String::new(),
            path: String::new(),
            results: Vec::new(),
            searches: snapshots,
            match_count: 0,
            files_searched: 0,
            error_count: 0,
            errors: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
            completed: true,
            success: true,
            exit_code: None,
            error: None,
        })
    }

    /// Kill a search and cleanup all resources
    pub async fn kill_search(
        &self,
        connection_id: &str,
        search_id: u32,
    ) -> Result<FsSearchOutput> {
        let start = std::time::Instant::now();
        let key = (connection_id.to_string(), search_id);
        let mut searches = self.searches.lock().await;

        if let Some(session) = searches.remove(&key) {
            session.cancel().await?;

            Ok(FsSearchOutput {
                search: Some(search_id),
                output: format!("Search {} cancelled and resources cleaned up", search_id),
                pattern: String::new(),
                path: String::new(),
                results: Vec::new(),
                searches: Vec::new(),
                match_count: 0,
                files_searched: 0,
                error_count: 0,
                errors: Vec::new(),
                duration_ms: start.elapsed().as_millis() as u64,
                completed: true,
                success: true,
                exit_code: Some(130), // SIGINT exit code
                error: None,
            })
        } else {
            Err(anyhow!(
                "Search {} not found for connection {}",
                search_id,
                connection_id
            ))
        }
    }

    /// Cleanup all searches for a connection (called on connection drop)
    pub async fn cleanup_connection(&self, connection_id: &str) -> usize {
        let mut searches = self.searches.lock().await;
        let to_remove: Vec<(String, u32)> = searches
            .keys()
            .filter(|(conn_id, _)| conn_id == connection_id)
            .cloned()
            .collect();

        let count = to_remove.len();
        for key in to_remove {
            if let Some(session) = searches.remove(&key) {
                log::debug!("Cleaning up search {} for connection {}", key.1, connection_id);
                let _ = session.cancel().await;
            }
        }
        count
    }
}

impl Default for SearchRegistry {
    fn default() -> Self {
        Self::new()
    }
}
