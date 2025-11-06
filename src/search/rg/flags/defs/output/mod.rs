//! Output category flags.
//!
//! This module contains all flags related to output formatting, display,
//! colors, separators, and output modes.

mod context_flags;
mod separator_flags;
mod display_flags;
mod color_flags;
mod output_mode_flags;
mod limit_flags;
mod help_flag;

// Re-export all flag types
pub(in crate::search::rg::flags) use context_flags::{
    AfterContext, BeforeContext, Context, ContextSeparator,
};
pub(in crate::search::rg::flags) use separator_flags::{
    FieldContextSeparator, FieldMatchSeparator, PathSeparator,
};
pub(in crate::search::rg::flags) use display_flags::{
    ByteOffset, Column, Heading, LineNumber, LineNumberNo,
    WithFilename, WithFilenameNo,
};
pub(in crate::search::rg::flags) use color_flags::{
    Color, Colors, HostnameBin, HyperlinkFormat,
};
pub(in crate::search::rg::flags) use output_mode_flags::{
    Null, OnlyMatching, Quiet, Replace, Trim, Vimgrep,
};
pub(in crate::search::rg::flags) use limit_flags::{
    IncludeZero, MaxColumns, MaxColumnsPreview,
};
pub(in crate::search::rg::flags) use help_flag::Help;
