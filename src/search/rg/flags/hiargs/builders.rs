/*!
Builder methods for HiArgs.

Contains methods for constructing haystacks, searchers, printers, and workers.
*/

use super::HiArgs;
use crate::search::rg::{
    flags::lowargs::{ContextMode, EncodingMode, Mode, SearchMode},
    haystack::HaystackBuilder,
    search::{PatternMatcher, Printer, SearchWorker, SearchWorkerBuilder},
};

impl HiArgs {
    /// Return a properly configured builder for constructing haystacks.
    ///
    /// The builder can be used to turn a directory entry (from the `ignore`
    /// crate) into something that can be searched.
    pub(crate) fn haystack_builder(&self) -> HaystackBuilder {
        let mut builder = HaystackBuilder::new();
        builder.strip_dot_prefix(self.paths.has_implicit_path);
        builder
    }

    /// Returns a builder for constructing a "path printer."
    ///
    /// This is useful for the `--files` mode in ripgrep, where the printer
    /// just needs to emit paths and not need to worry about the functionality
    /// of searching.
    pub(crate) fn path_printer_builder(&self) -> grep::printer::PathPrinterBuilder {
        let mut builder = grep::printer::PathPrinterBuilder::new();
        builder
            .color_specs(self.colors.clone())
            .hyperlink(self.hyperlink_config.clone())
            .separator(self.path_separator)
            .terminator(self.path_terminator.unwrap_or(b'\n'));
        builder
    }

    /// Returns a JSON printer for MCP output.
    ///
    /// Always returns JSON format since this is used for MCP protocol.
    pub(crate) fn printer<W: std::io::Write>(
        &self,
        _search_mode: SearchMode,
        wtr: W,
    ) -> Printer<W> {
        Printer::Json(self.printer_json(wtr))
    }

    /// Builds a JSON printer.
    pub(crate) fn printer_json<W: std::io::Write>(&self, wtr: W) -> grep::printer::JSON<W> {
        grep::printer::JSONBuilder::new()
            .pretty(false)
            .always_begin_end(false)
            .replacement(self.replace.clone().map(std::convert::Into::into))
            .build(wtr)
    }

    /// Build a worker for executing searches.
    ///
    /// Search results are found using the given matcher and written to the
    /// given printer.
    pub(crate) fn search_worker<W: std::io::Write>(
        &self,
        matcher: PatternMatcher,
        searcher: grep::searcher::Searcher,
        printer: Printer<W>,
    ) -> anyhow::Result<SearchWorker<W>> {
        let mut builder = SearchWorkerBuilder::new();
        builder
            .preprocessor(self.pre.clone())?
            .preprocessor_globs(self.pre_globs.clone())
            .search_zip(self.search_zip)
            .binary_detection_explicit(self.binary.explicit.clone())
            .binary_detection_implicit(self.binary.implicit.clone());
        Ok(builder.build(matcher, searcher, printer))
    }

    /// Build a searcher from the command line parameters.
    pub(crate) fn searcher(&self) -> anyhow::Result<grep::searcher::Searcher> {
        let line_term = if self.crlf {
            grep::matcher::LineTerminator::crlf()
        } else if self.null_data {
            grep::matcher::LineTerminator::byte(b'\x00')
        } else {
            grep::matcher::LineTerminator::byte(b'\n')
        };
        let mut builder = grep::searcher::SearcherBuilder::new();
        builder
            .max_matches(self.max_count)
            .line_terminator(line_term)
            .invert_match(self.invert_match)
            .line_number(self.line_number)
            .multi_line(self.multiline)
            .memory_map(self.mmap_choice.clone())
            .stop_on_nonmatch(self.stop_on_nonmatch);
        match self.context {
            ContextMode::Limited(ref limited) => {
                let (before, after) = limited.get();
                builder.before_context(before);
                builder.after_context(after);
            }
        }
        match self.encoding {
            EncodingMode::Auto => {} // default for the searcher
            EncodingMode::Some(ref enc) => {
                builder.encoding(Some(enc.clone()));
            }
            EncodingMode::Disabled => {
                builder.bom_sniffing(false);
            }
        }
        Ok(builder.build())
    }

    /// Create a new builder for recursive directory traversal.
    ///
    /// The builder returned can be used to start a single threaded or multi
    /// threaded directory traversal. For multi threaded traversal, the number
    /// of threads configured is equivalent to `HiArgs::threads`.
    ///
    /// If `HiArgs::threads` is equal to `1`, then callers should generally
    /// choose to explicitly use single threaded traversal since it won't have
    /// the unnecessary overhead of synchronization.
    pub(crate) fn walk_builder(&self) -> anyhow::Result<ignore::WalkBuilder> {
        let mut builder = ignore::WalkBuilder::new(&self.paths.paths[0]);
        for path in self.paths.paths.iter().skip(1) {
            builder.add(path);
        }
        if !self.no_ignore_files {
            for path in &self.ignore_file {
                // Silently skip ignore files with errors (non-fatal)
                let _ = builder.add_ignore(path);
            }
        }
        builder
            .max_depth(self.max_depth)
            .follow_links(self.follow)
            .max_filesize(self.max_filesize)
            .threads(self.threads)
            .same_file_system(self.one_file_system)
            .skip_stdout(matches!(self.mode, Mode::Search(_)))
            .overrides(self.globs.clone())
            .types(self.types.clone())
            .hidden(!self.hidden)
            .parents(!self.no_ignore_parent)
            .ignore(!self.no_ignore_dot)
            .git_global(!self.no_ignore_vcs && !self.no_ignore_global)
            .git_ignore(!self.no_ignore_vcs)
            .git_exclude(!self.no_ignore_vcs && !self.no_ignore_exclude)
            .require_git(!self.no_require_git)
            .ignore_case_insensitive(self.ignore_file_case_insensitive);
        if !self.no_ignore_dot {
            builder.add_custom_ignore_filename(".rgignore");
        }
        // REMOVED: Sort logic - dead ripgrep code, real sorting uses sorting.rs
        Ok(builder)
    }
}
