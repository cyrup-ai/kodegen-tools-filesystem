//!
//! Parallel visitor builder for files mode

use super::visitor::FilesListerVisitor;
use super::super::config::{RESULT_BUFFER_SIZE};
use super::super::context::SearchContext;

use ignore::ParallelVisitorBuilder;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::RwLock;

use super::super::super::types::{SearchError, SearchResult};

/// Parallel visitor builder for files mode
pub(super) struct FilesListerBuilder {
    pub(super) max_results: usize,
    pub(super) results: Arc<RwLock<Vec<SearchResult>>>,
    pub(super) total_matches: Arc<AtomicUsize>,
    pub(super) total_files: Arc<AtomicUsize>,
    pub(super) error_count: Arc<AtomicUsize>,
    pub(super) errors: Arc<RwLock<Vec<SearchError>>>,
}

impl<'s> ParallelVisitorBuilder<'s> for FilesListerBuilder {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(FilesListerVisitor {
            max_results: self.max_results,
            results: Arc::clone(&self.results),
            total_matches: Arc::clone(&self.total_matches),
            total_files: Arc::clone(&self.total_files),
            error_count: Arc::clone(&self.error_count),
            errors: Arc::clone(&self.errors),
            buffer: Vec::with_capacity(RESULT_BUFFER_SIZE),
        })
    }
}

impl FilesListerBuilder {
    pub(super) fn new(max_results: usize, ctx: &SearchContext) -> Self {
        Self {
            max_results,
            results: Arc::clone(&ctx.results),
            total_matches: Arc::clone(&ctx.total_matches),
            total_files: Arc::clone(&ctx.total_files),
            error_count: Arc::clone(&ctx.error_count),
            errors: Arc::clone(&ctx.errors),
        }
    }
}
