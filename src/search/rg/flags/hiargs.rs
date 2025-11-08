/*!
Provides the definition of high level arguments from CLI flags.
*/

use std::path::PathBuf;

use bstr::BString;

use crate::search::rg::{
    flags::lowargs::{
        BoundaryMode, CaseMode, ContextMode,
        EncodingMode, Engine, LowArgs, MmapMode,
        Mode, SearchMode,
    },
};

// Submodules containing decomposed functionality
mod builders;
mod helpers;
mod matchers;
mod tests;
mod types;

// Re-export types for external use
pub(crate) use types::{BinaryDetection, Paths, Patterns, State};

/// A high level representation of CLI arguments.
///
/// The distinction between low and high level arguments is somewhat arbitrary
/// and wishy washy. The main idea here is that high level arguments generally
/// require all of CLI parsing to be finished. For example, one cannot
/// construct a glob matcher until all of the glob patterns are known.
///
/// So while low level arguments are collected during parsing itself, high
/// level arguments aren't created until parsing has completely finished.
///
/// NOTE: Many fields/methods are unused but kept for ripgrep compatibility.
/// This struct represents the full ripgrep configuration surface.
#[derive(Debug)]
pub(crate) struct HiArgs {
    binary: BinaryDetection,
    boundary: Option<BoundaryMode>,
    case: CaseMode,
    context: ContextMode,
    crlf: bool,
    dfa_size_limit: Option<usize>,
    encoding: EncodingMode,
    engine: Engine,
    fixed_strings: bool,
    pub(crate) hidden: bool,
    invert_match: bool,
    line_number: bool,
    max_count: Option<u64>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) max_filesize: Option<u64>,
    mmap_choice: grep::searcher::MmapChoice,
    multiline: bool,
    multiline_dotall: bool,
    pub(crate) no_ignore_dot: bool,
    pub(crate) no_ignore_exclude: bool,
    pub(crate) no_ignore_global: bool,
    pub(crate) no_ignore_parent: bool,
    pub(crate) no_ignore_vcs: bool,
    no_unicode: bool,
    null_data: bool,
    patterns: Patterns,
    pre: Option<PathBuf>,
    pre_globs: ignore::overrides::Override,
    regex_size_limit: Option<usize>,
    replace: Option<BString>,
    search_zip: bool,
    stop_on_nonmatch: bool,
    types: ignore::types::Types,
}

impl HiArgs {
    /// Convert low level arguments into high level arguments.
    ///
    /// This process can fail for a variety of reasons. For example, invalid
    /// globs or some kind of environment issue.
    pub(crate) fn from_low_args(mut low: LowArgs) -> anyhow::Result<HiArgs> {
        // We modify the mode in-place on `low` so that subsequent conversions
        // see the correct mode.
        if let Mode::Search(ref mut mode) = low.mode {
            match *mode {
                // treat `-v --count-matches` as `-v --count`
                SearchMode::CountMatches if low.invert_match => {
                    *mode = SearchMode::Count;
                }
                // treat `-o --count` as `--count-matches`
                SearchMode::Count if low.only_matching => {
                    *mode = SearchMode::CountMatches;
                }
                _ => {}
            }
        }

        let mut state = State::new()?;
        let patterns = Patterns::from_low_args(&mut state, &mut low)?;
        let paths = Paths::from_low_args(&mut state, &patterns, &mut low)?;

        let binary = BinaryDetection::from_low_args(&state, &low);
        let types = helpers::types(&low)?;
        let pre_globs = helpers::preprocessor_globs(&state, &low)?;

        let line_number = low.line_number.unwrap_or(false);

        let mmap_choice = {
            // SAFETY: Memory maps are difficult to impossible to encapsulate
            // safely in a portable way that doesn't simultaneously negate some
            // of the benfits of using memory maps. For ripgrep's use, we never
            // mutate a memory map and generally never store the contents of
            // memory map in a data structure that depends on immutability.
            // Generally speaking, the worst thing that can happen is a SIGBUS
            // (if the underlying file is truncated while reading it), which
            // will cause ripgrep to abort. This reasoning should be treated as
            // suspect.
            let maybe = unsafe { grep::searcher::MmapChoice::auto() };
            let never = grep::searcher::MmapChoice::never();
            match low.mmap {
                MmapMode::Auto => {
                    if paths.paths.len() <= 10 && paths.paths.iter().all(|p| p.is_file()) {
                        // If we're only searching a few paths and all of them
                        // are files, then memory maps are probably faster.
                        maybe
                    } else {
                        never
                    }
                }
            }
        };

        Ok(HiArgs {
            patterns,
            binary,
            boundary: low.boundary,
            case: low.case,
            context: low.context,
            crlf: low.crlf,
            dfa_size_limit: low.dfa_size_limit,
            encoding: low.encoding,
            engine: low.engine,
            fixed_strings: low.fixed_strings,
            hidden: low.hidden,
            invert_match: low.invert_match,
            line_number,
            max_count: low.max_count,
            max_depth: low.max_depth,
            max_filesize: low.max_filesize,
            mmap_choice,
            multiline: low.multiline,
            multiline_dotall: low.multiline_dotall,
            no_ignore_dot: low.no_ignore_dot,
            no_ignore_exclude: low.no_ignore_exclude,
            no_ignore_global: low.no_ignore_global,
            no_ignore_parent: low.no_ignore_parent,
            no_ignore_vcs: low.no_ignore_vcs,
            no_unicode: low.no_unicode,
            null_data: low.null_data,
            pre: low.pre,
            pre_globs,
            regex_size_limit: low.regex_size_limit,
            replace: low.replace,
            search_zip: low.search_zip,
            stop_on_nonmatch: low.stop_on_nonmatch,
            types,
        })
    }

    /// Returns the file type matcher that was built.
    ///
    /// The matcher includes both the default rules and any rules added by the
    /// user for this specific invocation.
    pub(crate) fn types(&self) -> &ignore::types::Types {
        &self.types
    }
}
