//! Search category flags - decomposed into logical modules.
//!
//! This module re-exports all search-related flags from their respective submodules.

mod case_and_pattern_flags;
mod engine_flags;
mod encoding_unicode_flags;
mod multiline_boundary_flags;
mod limit_and_performance_flags;

// Re-export all flag structs for backward compatibility
pub(crate) use case_and_pattern_flags::{
    CaseSensitive, IgnoreCase, SmartCase,
    FixedStrings, InvertMatch,
};

pub(crate) use engine_flags::{
    AutoHybridRegex, EngineFlag as Engine, PCRE2,
};

pub(crate) use encoding_unicode_flags::{
    Encoding, NoUnicode, NoPcre2Unicode,
};

pub(crate) use multiline_boundary_flags::{
    Crlf, Multiline, MultilineDotall, NullData,
    LineRegexp, WordRegexp, StopOnNonmatch,
};

pub(crate) use limit_and_performance_flags::{
    DfaSizeLimit, MaxCount, RegexSizeLimit,
    Mmap, Threads, Text,
};
