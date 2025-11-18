//! Background task spawning and timeout monitoring

use super::super::super::types::{SearchSession, SearchSessionOptions, SearchIn};
use super::super::context::SearchContext;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, watch};

/// Spawn background search task using ripgrep libraries
pub fn spawn_search_task(
    search_id: String,
    options: SearchSessionOptions,
    root: PathBuf,
    cancellation_rx: watch::Receiver<bool>,
    sessions: Arc<RwLock<HashMap<String, SearchSession>>>,
) {
    let timeout_duration = options.timeout_ms.map(Duration::from_millis);

    // Spawn the actual search task
    let search_handle = tokio::task::spawn_blocking({
        let sessions = Arc::clone(&sessions);
        let search_id = search_id.clone();
        move || {
            // Get session references and create context
            let (mut ctx, search_in) = {
                let sessions_guard = sessions.blocking_read();
                if let Some(session) = sessions_guard.get(&search_id) {
                    let ctx = SearchContext::from_session(session, cancellation_rx);
                    (ctx, session.search_in)
                } else {
                    return; // Session not found
                }
            };

            // Branch based on search type
            if search_in == SearchIn::Content {
                super::super::content_search::execute(&options, &root, &mut ctx);
            } else {
                super::super::file_search::execute(&options, &root, &mut ctx);
            }
        }
    });

    // If timeout is specified, spawn a monitoring task
    if let Some(timeout) = timeout_duration {
        tokio::spawn(async move {
            // Wait for either search completion or timeout
            let timeout_result = tokio::time::timeout(timeout, search_handle).await;

            match timeout_result {
                Ok(_) => {
                    // Search completed before timeout - nothing to do
                }
                Err(_elapsed) => {
                    // Timeout occurred - send cancellation signal
                    log::warn!("Search session {search_id} timed out");

                    let sessions_guard = sessions.read().await;
                    if let Some(session) = sessions_guard.get(&search_id) {
                        // Only proceed if session still exists
                        let _ = session.cancellation_tx.send(true);

                        // Use try_write to avoid blocking
                        if let Ok(mut incomplete) = session.was_incomplete.try_write() {
                            *incomplete = true;
                        }
                    } else {
                        log::debug!("Timeout fired but session {search_id} already cleaned up");
                    }
                }
            }
        });
    } else {
        // No timeout - just detach the search task
        tokio::spawn(async move {
            let _ = search_handle.await;
        });
    }
}
