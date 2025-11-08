/*!
Ripgrep configuration types for MCP integration.

This module provides configuration types and structures that map ripgrep's
powerful search features to MCP tool arguments. Unlike the original ripgrep
which parses CLI flags, we receive structured JSON arguments from MCP clients
and use these types to configure the underlying grep-* libraries.
*/

// Submodules - types accessed via full paths
pub(crate) mod hiargs;
pub(crate) mod lowargs;

// Parse module - made public for testing but hidden from docs
#[doc(hidden)]
pub mod parse;

// Test utilities - public for integration tests but hidden from public docs
#[doc(hidden)]
pub use lowargs::{ColorChoice, ContextMode, ContextSeparator, LowArgs, Mode, SearchMode};
