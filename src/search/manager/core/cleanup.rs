//! Session cleanup and retention logic

use super::super::super::types::SearchSession;
use super::super::config::{
    ACTIVE_SESSION_RETENTION_SECS, CLEANUP_INTERVAL_SECS, COMPLETED_SESSION_RETENTION_SECS,
};

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Clean up old completed sessions with differentiated retention.
///
/// Removes sessions based on completion status:
/// - Completed searches: 30 seconds retention
/// - Active searches: 5 minutes retention
///
/// Recently-read sessions are preserved regardless of completion status.
pub async fn cleanup_sessions(sessions: &RwLock<HashMap<String, SearchSession>>) {
    let now = Instant::now();

    // Calculate different cutoff times for active vs completed searches
    let active_cutoff = now
        .checked_sub(Duration::from_secs(ACTIVE_SESSION_RETENTION_SECS))
        .unwrap_or(now);

    let completed_cutoff = now
        .checked_sub(Duration::from_secs(COMPLETED_SESSION_RETENTION_SECS))
        .unwrap_or(now);

    let mut sessions_guard = sessions.write().await;
    let initial_count = sessions_guard.len();

    sessions_guard.retain(|search_id, session| {
        // LOCK-FREE atomic loads
        let is_complete = session.is_complete.load(Ordering::Acquire);

        // Convert stored elapsed micros back to Instant
        let elapsed_micros = session.last_read_time_atomic.load(Ordering::Relaxed);
        let last_read = session
            .start_time
            .checked_add(Duration::from_micros(elapsed_micros))
            .unwrap_or(now);

        // Differentiated retention based on completion status
        let should_keep = if is_complete {
            // Completed searches: shorter retention (30 seconds)
            last_read > completed_cutoff
        } else {
            // Active searches: longer retention (5 minutes)
            last_read > active_cutoff
        };

        if !should_keep {
            let reason = if is_complete {
                "completed and inactive for 30s"
            } else {
                "active but no reads for 5min"
            };
            log::debug!("Cleaning up search session {search_id}: {reason}");
        }

        should_keep
    });

    let cleaned_count = initial_count - sessions_guard.len();
    if cleaned_count > 0 {
        log::info!("Cleaned up {cleaned_count} search sessions");
    }
}

/// Start background cleanup task (call once on manager creation)
///
/// Runs cleanup every minute with differentiated retention:
/// - Active searches: 5 minutes retention
/// - Completed searches: 30 seconds retention
pub fn start_cleanup_task(sessions: Arc<RwLock<HashMap<String, SearchSession>>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            cleanup_sessions(&sessions).await;
        }
    });
}
