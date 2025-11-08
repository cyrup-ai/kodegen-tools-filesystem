/*!
Builder methods for HiArgs.

Contains methods for constructing haystacks, searchers, printers, and workers.
*/

use super::HiArgs;
use crate::search::rg::{
    flags::lowargs::{ContextMode, EncodingMode, SearchMode},
    search::{PatternMatcher, Printer, SearchWorker, SearchWorkerBuilder},
};

impl HiArgs {
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
}
