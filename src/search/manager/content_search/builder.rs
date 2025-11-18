//! Builder for content search parallel visitors

use super::super::super::types::{SearchError, ReturnMode, SearchResult};
use super::super::config::RESULT_BUFFER_SIZE;
use super::{ContentSearchVisitor, ErrorVisitor};
use crate::search::rg::PatternMatcher;
use ignore::ParallelVisitorBuilder;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Parallel visitor builder for content search
pub(in super::super) struct ContentSearchBuilder {
    pub(super) hi_args: Arc<super::super::super::rg::flags::hiargs::HiArgs>,
    pub(super) matcher: PatternMatcher,
    pub(super) max_results: Option<usize>,
    pub(super) return_only: ReturnMode,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    // Deduplication for Paths mode
    pub(super) seen_files: Arc<RwLock<HashSet<String>>>,
    // Aggregation for Counts mode
    pub(super) file_counts: Arc<RwLock<HashMap<String, super::super::super::types::FileCountData>>>,
    pub(super) start_time: Instant,
}

impl<'s> ParallelVisitorBuilder<'s> for ContentSearchBuilder {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        use super::super::super::rg::flags::lowargs::SearchMode;

        // Clone Arc (cheap: just pointer copy)
        let hi_args = Arc::clone(&self.hi_args);

        // Clone matcher (cheap: regex already compiled)
        let matcher = self.matcher.clone();

        // Build searcher with error handling
        let searcher = match hi_args.searcher() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to build searcher: {e}");
                return Box::new(ErrorVisitor {
                    error_message: format!("Searcher initialization failed: {e}"),
                    error_count: Arc::clone(&self.error_count),
                    errors: Arc::clone(&self.errors),
                    was_incomplete: Arc::clone(&self.was_incomplete),
                });
            }
        };

        // Create thread-local buffer for JSON output
        let buffer = Vec::with_capacity(8192);

        // Build printer with thread-local buffer
        let printer = hi_args.printer(SearchMode::Json, buffer);

        // Build SearchWorker with error handling
        let worker = match hi_args.search_worker(matcher, searcher, printer) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to build search worker: {e}");
                return Box::new(ErrorVisitor {
                    error_message: format!("SearchWorker initialization failed: {e}"),
                    error_count: Arc::clone(&self.error_count),
                    errors: Arc::clone(&self.errors),
                    was_incomplete: Arc::clone(&self.was_incomplete),
                });
            }
        };

        // Build HaystackBuilder
        let haystack_builder = super::super::super::rg::haystack::HaystackBuilder::new();

        Box::new(ContentSearchVisitor {
            worker,
            haystack_builder,
            max_results: self.max_results,
            return_only: self.return_only,
            results: Arc::clone(&self.results),
            total_matches: Arc::clone(&self.total_matches),
            total_files: Arc::clone(&self.total_files),
            last_read_time_atomic: Arc::clone(&self.last_read_time_atomic),
            cancellation_rx: self.cancellation_rx.clone(),
            first_result_tx: self.first_result_tx.clone(),
            was_incomplete: Arc::clone(&self.was_incomplete),
            error_count: Arc::clone(&self.error_count),
            errors: Arc::clone(&self.errors),
            seen_files: Arc::clone(&self.seen_files),
            file_counts: Arc::clone(&self.file_counts),
            start_time: self.start_time,
            buffer: Vec::with_capacity(RESULT_BUFFER_SIZE),
            last_update_time: Instant::now(),
            matches_since_update: 0,
        })
    }
}
