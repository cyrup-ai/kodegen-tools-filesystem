/*!
Type definitions for high-level arguments.

Contains State, Patterns, Paths, and BinaryDetection types used by HiArgs.
*/

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::search::rg::flags::lowargs::{BinaryMode, LowArgs, Mode, PatternSource};

/// State that only needs to be computed once during argument parsing.
///
/// This state is meant to be somewhat generic and shared across multiple
/// low->high argument conversions. The state can even be mutated by various
/// conversions as a way to communicate changes to other conversions. For
/// example, reading patterns might consume from stdin. If we know stdin
/// has been consumed and no other file paths have been given, then we know
/// for sure that we should search the CWD. In this way, a state change
/// when reading the patterns can impact how the file paths are ultimately
/// generated.
#[derive(Debug)]
pub struct State {
    /// Whether stdin has already been consumed. This is useful to know and for
    /// providing good error messages when the user has tried to read from stdin
    /// in two different places. For example, `rg -f - -`.
    pub(crate) stdin_consumed: bool,
    /// The current working directory.
    pub(crate) cwd: PathBuf,
}

impl State {
    /// Initialize state to some sensible defaults.
    ///
    /// Note that the state values may change throughout the lifetime of
    /// argument parsing.
    pub fn new() -> anyhow::Result<State> {
        Ok(State {
            stdin_consumed: false,
            cwd: super::helpers::current_dir()?,
        })
    }
}

/// The disjunction of patterns to search for.
///
/// The number of patterns can be empty, e.g., via `-f /dev/null`.
#[derive(Debug)]
pub struct Patterns {
    /// The actual patterns to match.
    pub(crate) patterns: Vec<String>,
}

impl Patterns {
    /// Pulls the patterns out of the low arguments.
    ///
    /// This includes collecting patterns from -e/--regexp and -f/--file.
    ///
    /// If the invocation implies that the first positional argument is a
    /// pattern (the common case), then the first positional argument is
    /// extracted as well.
    pub(crate) fn from_low_args(_state: &mut State, low: &mut LowArgs) -> anyhow::Result<Patterns> {
        // The first positional is only a pattern when ripgrep is instructed to
        // search and neither -e/--regexp nor -f/--file is given. Basically,
        // the first positional is a pattern only when a pattern hasn't been
        // given in some other way.

        // No search means no patterns. Even if -e/--regexp or -f/--file is
        // given, we know we won't use them so don't bother collecting them.
        if !matches!(low.mode, Mode::Search(_)) {
            return Ok(Patterns { patterns: vec![] });
        }
        // If we got nothing from -e/--regexp and -f/--file, then the first
        // positional is a pattern.
        if low.patterns.is_empty() {
            anyhow::ensure!(
                !low.positional.is_empty(),
                "ripgrep requires at least one pattern to execute a search"
            );
            let ospat = low.positional.remove(0);
            let Ok(pat) = ospat.into_string() else {
                anyhow::bail!("pattern given is not valid UTF-8")
            };
            return Ok(Patterns {
                patterns: vec![pat],
            });
        }
        // Otherwise, we need to slurp up our patterns from -e/--regexp and
        // -f/--file. We de-duplicate as we go. If we don't de-duplicate,
        // then it can actually lead to major slow downs for sloppy inputs.
        // This might be surprising, and the regex engine will eventually
        // de-duplicate duplicative branches in a single regex (maybe), but
        // not until after it has gone through parsing and some other layers.
        // If there are a lot of duplicates, then that can lead to a sizeable
        // extra cost. It is lamentable that we pay the extra cost here to
        // de-duplicate for a likely uncommon case, but I've seen this have a
        // big impact on real world data.
        let mut seen = HashSet::new();
        let mut patterns = Vec::with_capacity(low.patterns.len());
        let mut add = |pat: String| {
            if !seen.contains(&pat) {
                seen.insert(pat.clone());
                patterns.push(pat);
            }
        };
        for source in low.patterns.drain(..) {
            match source {
                PatternSource::Regexp(pat) => add(pat),
            }
        }
        Ok(Patterns { patterns })
    }
}

/// The collection of paths we want to search for.
///
/// This guarantees that there is always at least one path.
#[derive(Debug)]
pub struct Paths {
    /// The actual paths.
    pub(crate) paths: Vec<PathBuf>,
}

impl Paths {
    /// Drain the search paths out of the given low arguments.
    pub(crate) fn from_low_args(
        state: &mut State,
        _: &Patterns,
        low: &mut LowArgs,
    ) -> anyhow::Result<Paths> {
        // We require a `&Patterns` even though we don't use it to ensure that
        // patterns have already been read from LowArgs. This let's us safely
        // assume that all remaining positional arguments are intended to be
        // file paths.

        let mut paths = Vec::with_capacity(low.positional.len());
        for osarg in low.positional.drain(..) {
            let path = PathBuf::from(osarg);
            if state.stdin_consumed && path == Path::new("-") {
                anyhow::bail!(
                    "error: attempted to read patterns from stdin \
                     while also searching stdin",
                );
            }
            paths.push(path);
        }
        log::debug!("number of paths given to search: {}", paths.len());
        if !paths.is_empty() {
            return Ok(Paths {
                paths,
            });
        }
        // N.B. is_readable_stdin is a heuristic! Part of the issue is that a
        // lot of "exec process" APIs will open a stdin pipe even though stdin
        // isn't really being used. ripgrep then thinks it should search stdin
        // and one gets the appearance of it hanging. It's a terrible failure
        // mode, but there really is no good way to mitigate it. It's just a
        // consequence of letting the user type 'rg foo' and "guessing" that
        // they meant to search the CWD.
        let is_readable_stdin = grep::cli::is_readable_stdin();
        let use_cwd =
            !is_readable_stdin || state.stdin_consumed || !matches!(low.mode, Mode::Search(_));
        log::debug!(
            "using heuristics to determine whether to read from \
             stdin or search ./ (\
             is_readable_stdin={is_readable_stdin}, \
             stdin_consumed={stdin_consumed}, \
             mode={mode:?})",
            stdin_consumed = state.stdin_consumed,
            mode = low.mode,
        );
        let path = if use_cwd {
            log::debug!("heuristic chose to search ./");
            PathBuf::from("./")
        } else {
            log::debug!("heuristic chose to search stdin");
            PathBuf::from("-")
        };
        Ok(Paths {
            paths: vec![path],
        })
    }
}

/// The "binary detection" configuration that ripgrep should use.
///
/// ripgrep actually uses two different binary detection heuristics depending
/// on whether a file is explicitly being searched (e.g., via a CLI argument)
/// or implicitly searched (e.g., via directory traversal). In general, the
/// former can never use a heuristic that lets it "quit" seaching before
/// either getting EOF or finding a match. (Because doing otherwise would be
/// considered a filter, and ripgrep follows the rule that an explicitly given
/// file is always searched.)
#[derive(Debug)]
pub struct BinaryDetection {
    pub explicit: grep::searcher::BinaryDetection,
    pub implicit: grep::searcher::BinaryDetection,
}

impl BinaryDetection {
    /// Determines the correct binary detection mode from low-level arguments.
    pub fn from_low_args(_: &State, low: &LowArgs) -> BinaryDetection {
        let none = matches!(low.binary, BinaryMode::AsText) || low.null_data;
        let convert = matches!(low.binary, BinaryMode::SearchAndSuppress);
        let explicit = if none {
            grep::searcher::BinaryDetection::none()
        } else {
            grep::searcher::BinaryDetection::convert(b'\x00')
        };
        let implicit = if none {
            grep::searcher::BinaryDetection::none()
        } else if convert {
            grep::searcher::BinaryDetection::convert(b'\x00')
        } else {
            grep::searcher::BinaryDetection::quit(b'\x00')
        };
        BinaryDetection { explicit, implicit }
    }

    /// Returns true when both implicit and explicit binary detection is
    /// disabled.
    pub fn is_none(&self) -> bool {
        let none = grep::searcher::BinaryDetection::none();
        self.explicit == none && self.implicit == none
    }
}
