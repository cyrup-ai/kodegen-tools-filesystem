//!
//! Parallel visitor builder for files mode

use super::visitor::FilesListerVisitor;
use super::super::config::{RESULT_BUFFER_SIZE};
use super::super::context::SearchContext;

use ignore::ParallelVisitorBuilder;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

use super::super::super::types::{SearchError, SearchResult};

/// Parallel visitor builder for files mode
pub(super) struct FilesListerBuilder {
    pub(super) max_results: usize,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) last_read_time_atomic: Arc<AtomicU64>,
    pub(super) cancellation_rx: watch::Receiver<bool>,
    pub(super) first_result_tx: watch::Sender<bool>,
    pub(super) was_incomplete: Arc<RwLock<bool>>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
    pub(super) start_time: Instant,
}

impl<'s> ParallelVisitorBuilder<'s> for FilesListerBuilder {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(FilesListerVisitor {
            max_results: self.max_results,
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

impl FilesListerBuilder {
    pub(super) fn new(max_results: usize, ctx: &SearchContext) -> Self {
        Self {
            max_results,
            results: Arc::clone(&ctx.results),
            total_matches: Arc::clone(&ctx.total_matches),
            last_read_time_atomic: Arc::clone(&ctx.last_read_time_atomic),
            cancellation_rx: ctx.cancellation_rx.clone(),
            first_result_tx: ctx.first_result_tx.clone(),
            was_incomplete: Arc::clone(&ctx.was_incomplete),
            error_count: Arc::clone(&ctx.error_count),
            errors: Arc::clone(&ctx.errors),
            start_time: ctx.start_time,
        }
    }
}
