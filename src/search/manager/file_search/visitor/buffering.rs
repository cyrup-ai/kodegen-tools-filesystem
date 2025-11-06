//! Buffer management for file search results

use crate::search::manager::config::{
    LAST_READ_UPDATE_INTERVAL_MS, LAST_READ_UPDATE_MATCH_THRESHOLD, RESULT_BUFFER_SIZE,
};
use crate::search::types::SearchResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{RwLock, watch};

/// Flush buffered results to shared storage
pub(super) fn flush_buffer(
    buffer: &mut Vec<SearchResult>,
    results: &Arc<RwLock<Vec<SearchResult>>>,
    first_result_tx: &watch::Sender<bool>,
    last_read_time_atomic: &Arc<AtomicU64>,
    start_time: &Instant,
) {
    if buffer.is_empty() {
        return;
    }

    // Check if this is the first batch of results
    let was_empty = results.blocking_read().is_empty();

    // Single lock acquisition for entire buffer
    {
        let mut results_guard = results.blocking_write();
        results_guard.extend(buffer.drain(..));
    }

    // Signal first result if this was the first batch
    if was_empty {
        let _ = first_result_tx.send(true);
    }

    // Update last read time once per flush
    {
        let elapsed_micros = start_time.elapsed().as_micros() as u64;
        last_read_time_atomic.store(elapsed_micros, Ordering::Relaxed);
    }
}

/// Add result to buffer, flush if full
pub(super) fn add_result(
    buffer: &mut Vec<SearchResult>,
    result: SearchResult,
    results: &Arc<RwLock<Vec<SearchResult>>>,
    first_result_tx: &watch::Sender<bool>,
    last_read_time_atomic: &Arc<AtomicU64>,
    start_time: &Instant,
) {
    buffer.push(result);

    if buffer.len() >= RESULT_BUFFER_SIZE {
        flush_buffer(buffer, results, first_result_tx, last_read_time_atomic, start_time);
    }
}

/// Update `last_read_time` if throttle threshold exceeded
pub(super) fn maybe_update_last_read_time(
    matches_since_update: &mut usize,
    last_update_time: &mut Instant,
    last_read_time_atomic: &Arc<AtomicU64>,
    start_time: &Instant,
) {
    *matches_since_update += 1;

    let now = Instant::now();
    let time_since_update = now.duration_since(*last_update_time);

    let should_update = time_since_update.as_millis() as u64 >= LAST_READ_UPDATE_INTERVAL_MS
        || *matches_since_update >= LAST_READ_UPDATE_MATCH_THRESHOLD;

    if should_update {
        let elapsed_micros = start_time.elapsed().as_micros() as u64;
        last_read_time_atomic.store(elapsed_micros, Ordering::Relaxed);
        *last_update_time = now;
        *matches_since_update = 0;
    }
}

/// Force update `last_read_time` (used in Drop)
pub(super) fn force_update_last_read_time(
    last_update_time: &mut Instant,
    matches_since_update: &mut usize,
    last_read_time_atomic: &Arc<AtomicU64>,
    start_time: &Instant,
) {
    let now = Instant::now();
    let elapsed_micros = start_time.elapsed().as_micros() as u64;
    last_read_time_atomic.store(elapsed_micros, Ordering::Relaxed);
    *last_update_time = now;
    *matches_since_update = 0;
}
