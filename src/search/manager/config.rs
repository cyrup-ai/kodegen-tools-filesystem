//! Configuration constants for search operations
//!
//! This module contains all tuning parameters for search performance and buffering.

/// Maximum number of detailed errors to track (prevents memory bloat)
pub(super) const MAX_DETAILED_ERRORS: usize = 100;

/// Size of thread-local result buffer before flushing to shared results
/// Larger = less lock contention, more memory per thread
/// Smaller = more frequent updates, more lock ops
pub(super) const RESULT_BUFFER_SIZE: usize = 50;

/// Default maximum results when client doesn't specify a limit
/// Covers 99% of typical use cases while preventing unbounded growth
pub(super) const DEFAULT_MAX_RESULTS: usize = 10_000;
