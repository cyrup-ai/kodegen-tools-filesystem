//!
//! This module provides file listing functionality for `Mode::Files`.
//! Lists all files that would be searched without actually searching.

mod visitor_builder;
mod visitor;
mod execute;

// Re-export the public API
pub use execute::execute;
