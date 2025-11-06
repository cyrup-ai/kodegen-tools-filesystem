/*!
Provides the definition of high level arguments from CLI flags.
*/

use std::path::PathBuf;

use bstr::BString;

use crate::search::rg::{
    flags::lowargs::{
        BoundaryMode, BufferMode, CaseMode, ColorChoice, ContextMode, ContextSeparator,
        EncodingMode, Engine, FieldContextSeparator, FieldMatchSeparator, LowArgs, MmapMode,
        Mode, SearchMode,
    },
    haystack::Haystack,
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
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct HiArgs {
    binary: BinaryDetection,
    boundary: Option<BoundaryMode>,
    buffer: BufferMode,
    byte_offset: bool,
    case: CaseMode,
    color: ColorChoice,
    colors: grep::printer::ColorSpecs,
    column: bool,
    context: ContextMode,
    context_separator: ContextSeparator,
    crlf: bool,
    dfa_size_limit: Option<usize>,
    encoding: EncodingMode,
    engine: Engine,
    field_context_separator: FieldContextSeparator,
    field_match_separator: FieldMatchSeparator,
    file_separator: Option<Vec<u8>>,
    fixed_strings: bool,
    follow: bool,
    globs: ignore::overrides::Override,
    heading: bool,
    pub(crate) hidden: bool,
    hyperlink_config: grep::printer::HyperlinkConfig,
    ignore_file_case_insensitive: bool,
    ignore_file: Vec<PathBuf>,
    include_zero: bool,
    invert_match: bool,
    is_terminal_stdout: bool,
    line_number: bool,
    max_columns: Option<u64>,
    max_columns_preview: bool,
    max_count: Option<u64>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) max_filesize: Option<u64>,
    mmap_choice: grep::searcher::MmapChoice,
    mode: Mode,
    multiline: bool,
    multiline_dotall: bool,
    pub(crate) no_ignore_dot: bool,
    pub(crate) no_ignore_exclude: bool,
    no_ignore_files: bool,
    pub(crate) no_ignore_global: bool,
    pub(crate) no_ignore_parent: bool,
    pub(crate) no_ignore_vcs: bool,
    no_require_git: bool,
    no_unicode: bool,
    null_data: bool,
    one_file_system: bool,
    only_matching: bool,
    path_separator: Option<u8>,
    paths: Paths,
    path_terminator: Option<u8>,
    patterns: Patterns,
    pre: Option<PathBuf>,
    pre_globs: ignore::overrides::Override,
    quiet: bool,
    quit_after_match: bool,
    regex_size_limit: Option<usize>,
    replace: Option<BString>,
    search_zip: bool,
    stats: Option<grep::printer::Stats>,
    stop_on_nonmatch: bool,
    threads: usize,
    trim: bool,
    types: ignore::types::Types,
    vimgrep: bool,
    with_filename: bool,
}

#[allow(dead_code)]
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
        let colors = helpers::take_color_specs(&mut state, &mut low);
        let hyperlink_config = helpers::take_hyperlink_config(&mut state, &mut low)?;
        let stats = helpers::stats(&low);
        let types = helpers::types(&low)?;
        let globs = helpers::globs(&state, &low)?;
        let pre_globs = helpers::preprocessor_globs(&state, &low)?;

        let color = match low.color {
            ColorChoice::Auto if !state.is_terminal_stdout => ColorChoice::Never,
            _ => low.color,
        };
        let column = low.column.unwrap_or(low.vimgrep);
        let heading = match low.heading {
            None => !low.vimgrep && state.is_terminal_stdout,
            Some(false) => false,
            Some(true) => !low.vimgrep,
        };
        let path_terminator = if low.null { Some(b'\x00') } else { None };
        let quit_after_match = stats.is_none() && low.quiet;
        let threads = if paths.is_one_file {
            1
        } else if let Some(threads) = low.threads {
            threads
        } else {
            std::thread::available_parallelism()
                .map_or(1, std::num::NonZero::get)
                .min(12)
        };
        log::debug!("using {threads} thread(s)");
        let with_filename = low
            .with_filename
            .unwrap_or(low.vimgrep || !paths.is_one_file);

        let file_separator = match low.mode {
            Mode::Search(SearchMode::Standard) => {
                if heading {
                    Some(b"".to_vec())
                } else {
                    let ContextMode::Limited(ref limited) = low.context;
                    let (before, after) = limited.get();
                    if before > 0 || after > 0 {
                        low.context_separator.clone().into_bytes()
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };

        let line_number = low.line_number.unwrap_or_else(|| {
            if low.quiet {
                return false;
            }
            let Mode::Search(ref search_mode) = low.mode else {
                return false;
            };
            match *search_mode {
                SearchMode::Count | SearchMode::CountMatches => false,
                SearchMode::Json => true,
                SearchMode::Standard => {
                    // A few things can imply counting line numbers. In
                    // particular, we generally want to show line numbers by
                    // default when printing to a tty for human consumption,
                    // except for one interesting case: when we're only
                    // searching stdin. This makes pipelines work as expected.
                    (state.is_terminal_stdout && !paths.is_only_stdin()) || column || low.vimgrep
                }
            }
        });

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
            mode: low.mode,
            patterns,
            paths,
            binary,
            boundary: low.boundary,
            buffer: low.buffer,
            byte_offset: low.byte_offset,
            case: low.case,
            color,
            colors,
            column,
            context: low.context,
            context_separator: low.context_separator,
            crlf: low.crlf,
            dfa_size_limit: low.dfa_size_limit,
            encoding: low.encoding,
            engine: low.engine,
            field_context_separator: low.field_context_separator,
            field_match_separator: low.field_match_separator,
            file_separator,
            fixed_strings: low.fixed_strings,
            follow: low.follow,
            heading,
            hidden: low.hidden,
            hyperlink_config,
            ignore_file: low.ignore_file,
            ignore_file_case_insensitive: low.ignore_file_case_insensitive,
            include_zero: low.include_zero,
            invert_match: low.invert_match,
            is_terminal_stdout: state.is_terminal_stdout,
            line_number,
            max_columns: low.max_columns,
            max_columns_preview: low.max_columns_preview,
            max_count: low.max_count,
            max_depth: low.max_depth,
            max_filesize: low.max_filesize,
            mmap_choice,
            multiline: low.multiline,
            multiline_dotall: low.multiline_dotall,
            no_ignore_dot: low.no_ignore_dot,
            no_ignore_exclude: low.no_ignore_exclude,
            no_ignore_files: low.no_ignore_files,
            no_ignore_global: low.no_ignore_global,
            no_ignore_parent: low.no_ignore_parent,
            no_ignore_vcs: low.no_ignore_vcs,
            no_require_git: low.no_require_git,
            no_unicode: low.no_unicode,
            null_data: low.null_data,
            one_file_system: low.one_file_system,
            only_matching: low.only_matching,
            globs,
            path_separator: low.path_separator,
            path_terminator,
            pre: low.pre,
            pre_globs,
            quiet: low.quiet,
            quit_after_match,
            regex_size_limit: low.regex_size_limit,
            replace: low.replace,
            search_zip: low.search_zip,
            stats,
            stop_on_nonmatch: low.stop_on_nonmatch,
            threads,
            trim: low.trim,
            types,
            vimgrep: low.vimgrep,
            with_filename,
        })
    }

    /// Returns true when ripgrep had to guess to search the current working
    /// directory. That is, it's true when ripgrep is called without any file
    /// paths or directories to search.
    ///
    /// Other than changing how file paths are printed (i.e., without the
    /// leading `./`), it's also useful to know for diagnostic reasons. For
    /// example, ripgrep will print an error message when nothing is searched
    /// since it's possible the ignore rules in play are too aggressive. But
    /// this warning is only emitted when ripgrep was called without any
    /// explicit file paths since otherwise the warning would likely be too
    /// aggressive.
    pub(crate) fn has_implicit_path(&self) -> bool {
        self.paths.has_implicit_path
    }

    /// Returns true if some non-zero number of matches is believed to be
    /// possible.
    ///
    /// When this returns false, it is impossible for ripgrep to ever report
    /// a match.
    pub(crate) fn matches_possible(&self) -> bool {
        if self.patterns.patterns.is_empty() && !self.invert_match {
            return false;
        }
        if self.max_count == Some(0) {
            return false;
        }
        true
    }

    /// Returns the "mode" that ripgrep should operate in.
    ///
    /// This is generally useful for determining what action ripgrep should
    /// take. The main mode is of course to "search," but there are other
    /// non-search modes such as `--type-list` and `--files`.
    pub(crate) fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns true if ripgrep should operate in "quiet" mode.
    ///
    /// Generally speaking, quiet mode means that ripgrep should not print
    /// anything to stdout. There are some exceptions. For example, when the
    /// user has provided `--stats`, then ripgrep will print statistics to
    /// stdout.
    pub(crate) fn quiet(&self) -> bool {
        self.quiet
    }

    /// Returns true when ripgrep should stop searching after a single match is
    /// found.
    ///
    /// This is useful for example when quiet mode is enabled. In that case,
    /// users generally can't tell the difference in behavior between a search
    /// that finds all matches and a search that only finds one of them. (An
    /// exception here is if `--stats` is given, then `quit_after_match` will
    /// always return false since the user expects ripgrep to find everything.)
    pub(crate) fn quit_after_match(&self) -> bool {
        self.quit_after_match
    }

    /// STUBBED: Dead ripgrep code - real sorting uses sorting.rs module.
    /// Returns haystacks unchanged since sorting is handled elsewhere.
    pub(crate) fn sort<'a, I>(&self, haystacks: I) -> Box<dyn Iterator<Item = Haystack> + 'a>
    where
        I: Iterator<Item = Haystack> + 'a,
    {
        // No sorting - real implementation uses sorting.rs
        Box::new(haystacks)
    }

    /// Returns a stats object if the user requested that ripgrep keep track
    /// of various metrics during a search.
    ///
    /// When this returns `None`, then callers may assume that the user did
    /// not request statistics.
    pub(crate) fn stats(&self) -> Option<grep::printer::Stats> {
        self.stats.clone()
    }

    /// Returns the total number of threads ripgrep should use to execute a
    /// search.
    ///
    /// This number is the result of reasoning about both heuristics (like
    /// the available number of cores) and whether ripgrep's mode supports
    /// parallelism. It is intended that this number be used to directly
    /// determine how many threads to spawn.
    pub(crate) fn threads(&self) -> usize {
        self.threads
    }

    /// Returns the file type matcher that was built.
    ///
    /// The matcher includes both the default rules and any rules added by the
    /// user for this specific invocation.
    pub(crate) fn types(&self) -> &ignore::types::Types {
        &self.types
    }
}
