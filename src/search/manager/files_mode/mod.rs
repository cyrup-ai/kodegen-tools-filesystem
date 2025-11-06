//!
//! This module provides file listing functionality for `Mode::Files`.
//! Lists all files that would be searched without actually searching.

mod visitor_builder;
mod visitor;
mod execute;

#[cfg(test)]
mod tests;

// Re-export the public API
pub(super) use execute::execute;
