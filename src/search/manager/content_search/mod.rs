//! Content search implementation using grep and parallel directory traversal
//!
//! This module provides the visitor and builder for searching file contents
//! using regex patterns with the `grep` and `ignore` crates.

mod builder;
mod error_visitor;
mod execute;
mod visitor_core;
mod visitor_impl;

pub(super) use builder::ContentSearchBuilder;
pub(super) use error_visitor::ErrorVisitor;
pub use execute::execute;
pub(super) use visitor_core::ContentSearchVisitor;
