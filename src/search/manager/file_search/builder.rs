//! Parallel visitor builder for file search

use super::visitor::FileSearchVisitor;
use crate::search::manager::config::RESULT_BUFFER_SIZE;
use crate::search::types::{CaseMode, SearchResult};
use ignore::{ParallelVisitor, ParallelVisitorBuilder};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor builder for file search
pub(super) struct FileSearchBuilder {
    pub(super) glob_pattern: Option<globset::GlobMatcher>,
    pub(super) pattern: String,
    pub(super) pattern_lower: String,
    pub(super) case_mode: CaseMode,
    pub(super) is_pattern_lowercase: bool,
    pub(super) word_boundary: bool,
    pub(super) max_results: usize,
    pub(super) early_termination: bool,
    pub(super) early_term_triggered: Arc<AtomicBool>,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<crate::search::types::SearchError>>>,
    pub(super) start_time: Instant,
}

impl<'s> ParallelVisitorBuilder<'s> for FileSearchBuilder {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(FileSearchVisitor {
            glob_pattern: self.glob_pattern.clone(),
            pattern: self.pattern.clone(),
            pattern_lower: self.pattern_lower.clone(),
            case_mode: self.case_mode,
            is_pattern_lowercase: self.is_pattern_lowercase,
            word_boundary: self.word_boundary,
            max_results: self.max_results,
            early_termination: self.early_termination,
            early_term_triggered: Arc::clone(&self.early_term_triggered),
            results: Arc::clone(&self.results),
            total_matches: Arc::clone(&self.total_matches),
            last_read_time_atomic: Arc::clone(&self.last_read_time_atomic),
            cancellation_rx: self.cancellation_rx.clone(),
            first_result_tx: self.first_result_tx.clone(),
            was_incomplete: Arc::clone(&self.was_incomplete),
            error_count: Arc::clone(&self.error_count),
            errors: Arc::clone(&self.errors),
            buffer: Vec::with_capacity(RESULT_BUFFER_SIZE),
            last_update_time: Instant::now(),
            matches_since_update: 0,
            start_time: self.start_time,
        })
    }
}
