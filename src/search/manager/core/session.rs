//! Session creation and initialization logic

use super::super::super::types::{SearchSession, SearchSessionOptions};
use super::super::config::{ABSOLUTE_MAX_RESULTS, DEFAULT_MAX_RESULTS};
use crate::validate_path;
use kodegen_mcp_tool::error::McpError;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};
use uuid::Uuid;

/// Enforce result limits on search options
///
/// Returns the effective max_results to use for the search
pub fn enforce_result_limits(options: &mut SearchSessionOptions) -> usize {
    match options.max_results {
        None => {
            // No limit specified - use default
            options.max_results = Some(DEFAULT_MAX_RESULTS as u32);
            DEFAULT_MAX_RESULTS
        }
        Some(requested) => {
            // Limit specified - cap at absolute maximum
            let capped = requested.min(ABSOLUTE_MAX_RESULTS as u32);
            if capped < requested {
                log::warn!(
                    "Search max_results capped from {requested} to {capped} (absolute limit)"
                );
            }
            options.max_results = Some(capped);
            capped as usize
        }
    }
}

/// Validate path and generate session ID
///
/// # Errors
/// Returns error if path validation fails
pub async fn validate_and_generate_id(
    options: &SearchSessionOptions,
    config_manager: &kodegen_config_manager::ConfigManager,
) -> Result<(PathBuf, String), McpError> {
    // Validate path FIRST (no point generating ID if path invalid)
    let validated_path = validate_path(&options.root_path, config_manager).await?;

    // Generate unique session ID - UUID v4 collisions are 1 in 5.3×10³⁶
    let session_id = Uuid::new_v4().to_string();

    Ok((validated_path, session_id))
}

/// Build a new search session
///
/// Returns the session ready for insertion into the sessions map
pub fn build_session(
    session_id: String,
    options: &SearchSessionOptions,
    effective_max_results: usize,
    cancellation_tx: watch::Sender<bool>,
    first_result_tx: watch::Sender<bool>,
) -> SearchSession {
    SearchSession {
        id: session_id.clone(),
        cancellation_tx: cancellation_tx.clone(),
        first_result_tx: first_result_tx.clone(),
        results: Arc::new(RwLock::new(Vec::new())),
        is_complete: Arc::new(AtomicBool::new(false)),
        is_error: Arc::new(RwLock::new(false)),
        error: Arc::new(RwLock::new(None)),
        total_matches: Arc::new(AtomicUsize::new(0)),
        total_files: Arc::new(AtomicUsize::new(0)),
        last_read_time_atomic: Arc::new(AtomicU64::new(0)),
        start_time: Instant::now(),
        was_incomplete: Arc::new(RwLock::new(false)),
        search_type: options.search_type.clone(),
        pattern: options.pattern.clone(),
        timeout_ms: options.timeout_ms,
        error_count: Arc::new(AtomicUsize::new(0)),
        errors: Arc::new(RwLock::new(Vec::new())),
        max_results: effective_max_results,
        output_mode: options.output_mode,
        seen_files: Arc::new(RwLock::new(HashSet::new())),
        file_counts: Arc::new(RwLock::new(HashMap::new())),
    }
}
