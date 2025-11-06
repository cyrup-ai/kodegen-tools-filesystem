/*!
Pattern matcher construction for HiArgs.

Contains logic for building Rust regex and PCRE2 matchers.
*/

use grep_pcre2;

use super::HiArgs;
use crate::search::rg::{
    flags::lowargs::{BoundaryMode, CaseMode, Engine},
    search::PatternMatcher,
};

impl HiArgs {
    /// Return the matcher that should be used for searching using the engine
    /// choice made by the user.
    ///
    /// If there was a problem building the matcher (e.g., a syntax error),
    /// then this returns an error.
    pub(crate) fn matcher(&self) -> anyhow::Result<PatternMatcher> {
        match self.engine {
            Engine::Default => match self.matcher_rust() {
                Ok(m) => Ok(m),
                Err(err) => {
                    anyhow::bail!(super::helpers::suggest_other_engine(err.to_string()));
                }
            },
            Engine::PCRE2 => Ok(self.matcher_pcre2()?),
            Engine::Auto => {
                let rust_err = match self.matcher_rust() {
                    Ok(m) => return Ok(m),
                    Err(err) => err,
                };
                log::debug!("error building Rust regex in hybrid mode:\n{rust_err}",);

                let pcre_err = match self.matcher_pcre2() {
                    Ok(m) => return Ok(m),
                    Err(err) => err,
                };
                let divider = "~".repeat(79);
                anyhow::bail!(
                    "regex could not be compiled with either the default \
                     regex engine or with PCRE2.\n\n\
                     default regex engine error:\n\
                     {divider}\n\
                     {rust_err}\n\
                     {divider}\n\n\
                     PCRE2 regex engine error:\n{pcre_err}",
                );
            }
        }
    }

    /// Build a matcher using PCRE2.
    ///
    /// If there was a problem building the matcher (such as a regex syntax
    /// error), then an error is returned.
    ///
    /// If the `pcre2` feature is not enabled then this always returns an
    /// error.
    pub(crate) fn matcher_pcre2(&self) -> anyhow::Result<PatternMatcher> {
        let mut builder = grep_pcre2::RegexMatcherBuilder::new();
        builder.multi_line(true).fixed_strings(self.fixed_strings);
        match self.case {
            CaseMode::Sensitive => {
                builder.caseless(false);
            }
            CaseMode::Insensitive => {
                builder.caseless(true);
            }
            CaseMode::Smart => {
                builder.case_smart(true);
            }
        }
        if let Some(ref boundary) = self.boundary {
            match *boundary {
                BoundaryMode::Line => {
                    builder.whole_line(true);
                }
                BoundaryMode::Word => {
                    builder.word(true);
                }
            }
        }
        // For whatever reason, the JIT craps out during regex compilation with
        // a "no more memory" error on 32 bit systems. So don't use it there.
        if cfg!(target_pointer_width = "64") {
            builder
                .jit_if_available(true)
                // The PCRE2 docs say that 32KB is the default, and that 1MB
                // should be big enough for anything. But let's crank it to
                // 10MB.
                .max_jit_stack_size(Some(10 * (1 << 20)));
        }
        if !self.no_unicode {
            builder.utf(true).ucp(true);
        }
        if self.multiline {
            builder.dotall(self.multiline_dotall);
        }
        if self.crlf {
            builder.crlf(true);
        }
        let m = builder.build_many(&self.patterns.patterns)?;
        Ok(PatternMatcher::PCRE2(m))
    }

    /// Build a matcher using Rust's regex engine.
    ///
    /// If there was a problem building the matcher (such as a regex syntax
    /// error), then an error is returned.
    pub(crate) fn matcher_rust(&self) -> anyhow::Result<PatternMatcher> {
        let mut builder = grep::regex::RegexMatcherBuilder::new();
        builder
            .multi_line(true)
            .unicode(!self.no_unicode)
            .octal(false)
            .fixed_strings(self.fixed_strings);
        match self.case {
            CaseMode::Sensitive => {
                log::debug!("Setting Rust matcher to case SENSITIVE");
                builder.case_insensitive(false);
            }
            CaseMode::Insensitive => {
                log::debug!("Setting Rust matcher to case INSENSITIVE");
                builder.case_insensitive(true);
            }
            CaseMode::Smart => {
                log::debug!("Setting Rust matcher to SMART case");
                builder.case_smart(true);
            }
        }
        if let Some(ref boundary) = self.boundary {
            match *boundary {
                BoundaryMode::Line => {
                    builder.whole_line(true);
                }
                BoundaryMode::Word => {
                    builder.word(true);
                }
            }
        }
        if self.multiline {
            builder.dot_matches_new_line(self.multiline_dotall);
            if self.crlf {
                builder.crlf(true).line_terminator(None);
            }
        } else {
            builder
                .line_terminator(Some(b'\n'))
                .dot_matches_new_line(false);
            if self.crlf {
                builder.crlf(true);
            }
            // We don't need to set this in multiline mode since multiline
            // matchers don't use optimizations related to line terminators.
            // Moreover, a multiline regex used with --null-data should
            // be allowed to match NUL bytes explicitly, which this would
            // otherwise forbid.
            if self.null_data {
                builder.line_terminator(Some(b'\x00'));
            }
        }
        if let Some(limit) = self.regex_size_limit {
            builder.size_limit(limit);
        }
        if let Some(limit) = self.dfa_size_limit {
            builder.dfa_size_limit(limit);
        }
        if !self.binary.is_none() {
            builder.ban_byte(Some(b'\x00'));
        }
        let m = match builder.build_many(&self.patterns.patterns) {
            Ok(m) => m,
            Err(err) => {
                anyhow::bail!(super::helpers::suggest_text(super::helpers::suggest_multiline(
                    err.to_string()
                )))
            }
        };
        Ok(PatternMatcher::RustRegex(m))
    }
}
