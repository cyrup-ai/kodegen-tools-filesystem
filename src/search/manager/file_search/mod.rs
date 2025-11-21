//! File search implementation using glob patterns and parallel directory traversal
//!
//! This module provides the visitor and builder for searching files by name
//! with support for glob patterns and exact matching.

mod builder;
pub(super) mod execute;
mod visitor;

// Re-export the execute function for easier access from parent module
pub use execute::execute;
