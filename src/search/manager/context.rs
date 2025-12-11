//! Search context for coordinating parallel search operations
//!
//! This module provides the `SearchContext` which holds all the state
//! needed during a search operation. Some fields use Arc for thread-safe
//! parallel access during directory traversal.

use super::super::types::{FileCountData, SearchError, ReturnMode, SearchResult};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Context for executing searches, containing all search state
/// 
/// Note: Some fields use Arc/RwLock for thread-safe parallel directory traversal.
/// These are NOT for cross-request session management - they're for parallelism
/// within a single blocking execute() call.
pub struct SearchContext {
    // === Parallel-access fields (wrapped for thread safety) ===
    /// Accumulated search results (Arc for parallel thread access)
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,

    /// Total matches found (atomic for parallel increment)
    pub(super) total_matches: Arc<AtomicUsize>,

    /// Total files searched (atomic for parallel increment)
    pub(super) total_files: Arc<AtomicUsize>,

    /// Error count (atomic for parallel increment)
    pub(super) error_count: Arc<AtomicUsize>,

    /// Detailed errors (RwLock for parallel write access)
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,

    /// Deduplication for Paths mode (RwLock for parallel write access)
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,

    /// Count aggregation for Counts mode (RwLock for parallel write access)
    pub(super) file_counts: Arc<RwLock<HashMap<String, FileCountData>>>,

    // === Plain fields (no parallel access needed) ===
    /// Maximum results to collect
    pub max_results: usize,

    /// What to return from search
    pub return_only: ReturnMode,

    /// Whether search completed
    pub is_complete: bool,

    /// Whether search encountered fatal error
    pub is_error: bool,

    /// Error message if any
    pub error: Option<String>,
    
    /// Client's working directory from ToolExecutionContext
    /// Used to resolve relative paths in ripgrep integration
    pub client_pwd: Option<PathBuf>,
    
    /// Pattern type detected during search (for filename search)
    /// Set by file_search::execute(), read by session.rs for output
    pub pattern_type: Option<crate::search::types::PatternMode>,
}

impl SearchContext {
    /// Create a new search context with optional client pwd
    #[must_use]
    pub fn new(
        max_results: usize,
        return_only: ReturnMode,
        client_pwd: Option<PathBuf>,
    ) -> Self {
        Self {
            results: Arc::new(RwLock::new(Vec::new())),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            seen_files: Arc::new(RwLock::new(HashSet::new())),
            file_counts: Arc::new(RwLock::new(HashMap::new())),
            max_results,
            return_only,
            is_complete: false,
            is_error: false,
            error: None,
            client_pwd,
            pattern_type: None,
        }
    }

    /// Take ownership of results, consuming the context
    #[must_use]
    pub fn take_results(self) -> Vec<SearchResult> {
        // Extract from Arc<RwLock<...>>
        match Arc::try_unwrap(self.results) {
            Ok(rwlock) => rwlock.into_inner(),
            Err(arc) => {
                // Arc still has multiple references (shouldn't happen in normal use)
                // Fall back to cloning
                arc.blocking_read().clone()
            }
        }
    }

    /// Get total matches count
    #[must_use]
    pub fn total_matches(&self) -> usize {
        self.total_matches.load(Ordering::SeqCst)
    }

    /// Get total files count
    #[must_use]
    pub fn total_files(&self) -> usize {
        self.total_files.load(Ordering::SeqCst)
    }

    /// Get error count
    #[must_use]
    pub fn error_count_value(&self) -> usize {
        self.error_count.load(Ordering::SeqCst)
    }

    /// Get reference to results (for reading without consuming)
    #[must_use]
    pub fn results(&self) -> &Arc<RwLock<Vec<SearchResult>>> {
        &self.results
    }

    /// Get reference to errors (for reading without consuming)
    #[must_use]
    pub fn errors(&self) -> &Arc<RwLock<Vec<SearchError>>> {
        &self.errors
    }
}
