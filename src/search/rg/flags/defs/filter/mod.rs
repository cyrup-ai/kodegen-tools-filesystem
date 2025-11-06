//! Filter category flags.
//!
//! This module is decomposed into logical submodules for maintainability:
//! - `binary_and_types` - Binary file and type filtering
//! - `path_and_globs` - Path pattern matching and globs
//! - `ignore_files` - Custom ignore file configuration
//! - `ignore_sources` - Ignore file source control
//! - `traversal_and_limits` - Directory traversal and size limits

// Submodules organized by functionality
mod binary_and_types;
mod path_and_globs;
mod ignore_files;
mod ignore_sources;
mod traversal_and_limits;

// Re-export all flag implementations for parent module
pub(super) use binary_and_types::{Binary, Type, TypeNot};
pub(super) use path_and_globs::{Follow, Glob, GlobCaseInsensitive, Hidden, IGlob};
pub(super) use ignore_files::{IgnoreFile, IgnoreFileCaseInsensitive, NoIgnoreFiles};
pub(super) use ignore_sources::{
    NoIgnore, NoIgnoreDot, NoIgnoreExclude, NoIgnoreGlobal,
    NoIgnoreParent, NoIgnoreVcs, NoRequireGit,
};
pub(super) use traversal_and_limits::{MaxDepth, MaxFilesize, OneFileSystem, Unrestricted};
