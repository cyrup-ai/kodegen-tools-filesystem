//! Sorting functionality for search results
//!
//! This module provides sorting capabilities for search results based on various criteria
//! such as file path, modification time, access time, and creation time.

use std::cmp::Ordering;
use std::time::SystemTime;

use super::types::SearchResult;

/// Sort criterion for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort alphabetically by file path
    Path,
    /// Sort by last modified time
    Modified,
    /// Sort by last accessed time
    Accessed,
    /// Sort by creation time
    Created,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order (oldest first for time, A-Z for path)
    Ascending,
    /// Descending order (newest first for time, Z-A for path)
    Descending,
}

/// Sort search results in place based on specified criterion and direction
///
/// # Arguments
/// * `results` - Mutable slice of search results to sort
/// * `sort_by` - The criterion to sort by
/// * `direction` - The sort direction (ascending or descending)
///
/// # Platform Notes
/// - `modified` timestamp is available on all platforms
/// - `accessed` timestamp is available on most platforms but may be None
/// - `created` timestamp is available on Windows, may be None on some Unix systems
///
/// Results with missing timestamps are sorted to the end (ascending) or beginning (descending)
pub fn sort_results(results: &mut [SearchResult], sort_by: SortBy, direction: SortDirection) {
    results.sort_by(|a, b| {
        let ordering = match sort_by {
            SortBy::Path => compare_paths(&a.file, &b.file),
            SortBy::Modified => compare_optional_times(&a.modified, &b.modified),
            SortBy::Accessed => compare_optional_times(&a.accessed, &b.accessed),
            SortBy::Created => compare_optional_times(&a.created, &b.created),
        };

        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

/// Compare two file paths alphabetically
fn compare_paths(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

/// Compare two optional timestamps
///
/// Ordering logic:
/// - Both Some: Compare normally
/// - a is Some, b is None: a comes first (has timestamp is "less")
/// - a is None, b is Some: b comes first
/// - Both None: Equal (maintain original order)
fn compare_optional_times(a: &Option<SystemTime>, b: &Option<SystemTime>) -> Ordering {
    match (a, b) {
        (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
        (Some(_), None) => Ordering::Less, // Has timestamp comes first
        (None, Some(_)) => Ordering::Greater, // Missing timestamp goes last
        (None, None) => Ordering::Equal,   // Both missing, maintain order
    }
}


