//! Multiline, boundary, and line terminator flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{BoundaryMode, LowArgs},
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// --crlf
#[derive(Debug)]
pub(super) struct Crlf;

impl Flag for Crlf {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "crlf"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-crlf")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use CRLF line terminators (nice for Windows)."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will treat CRLF (\fB\\r\\n\fP) as a line terminator
instead of just \fB\\n\fP.
.sp
Principally, this permits the line anchor assertions \fB^\fP and \fB$\fP in
regex patterns to treat CRLF, CR or LF as line terminators instead of just LF.
Note that they will never match between a CR and a LF. CRLF is treated as one
single line terminator.
.sp
When using the default regex engine, CRLF support can also be enabled inside
the pattern with the \fBR\fP flag. For example, \fB(?R:$)\fP will match just
before either CR or LF, but never between CR and LF.
.sp
This flag overrides \flag{null-data}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.crlf = v.unwrap_switch();
        if args.crlf {
            args.null_data = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_crlf() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--crlf"]).expect("Test parsing should succeed");
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--crlf", "--null-data"]).expect("Test parsing should succeed");
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).expect("Test parsing should succeed");
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).expect("Test parsing should succeed");
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf", "--no-crlf"]).expect("Test parsing should succeed");
    assert_eq!(false, args.crlf);
    assert_eq!(false, args.null_data);
}

/// -U/--multiline
#[derive(Debug)]
pub(super) struct Multiline;

impl Flag for Multiline {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'U')
    }
    fn name_long(&self) -> &'static str {
        "multiline"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable searching across multiple lines."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enable searching across multiple lines.
.sp
When multiline mode is enabled, ripgrep will lift the restriction that a
match cannot include a line terminator. For example, when multiline mode
is not enabled (the default), then the regex \fB\\p{any}\fP will match any
Unicode codepoint other than \fB\\n\fP. Similarly, the regex \fB\\n\fP is
explicitly forbidden, and if you try to use it, ripgrep will return an error.
However, when multiline mode is enabled, \fB\\p{any}\fP will match any Unicode
codepoint, including \fB\\n\fP, and regexes like \fB\\n\fP are permitted.
.sp
An important caveat is that multiline mode does not change the match semantics
of \fB.\fP. Namely, in most regex matchers, a \fB.\fP will by default match any
character other than \fB\\n\fP, and this is true in ripgrep as well. In order
to make \fB.\fP match \fB\\n\fP, you must enable the "dot all" flag inside the
regex. For example, both \fB(?s).\fP and \fB(?s:.)\fP have the same semantics,
where \fB.\fP will match any character, including \fB\\n\fP. Alternatively, the
\flag{multiline-dotall} flag may be passed to make the "dot all" behavior the
default. This flag only applies when multiline search is enabled.
.sp
There is no limit on the number of the lines that a single match can span.
.sp
\fBWARNING\fP: Because of how the underlying regex engine works, multiline
searches may be slower than normal line-oriented searches, and they may also
use more memory. In particular, when multiline mode is enabled, ripgrep
requires that each file it searches is laid out contiguously in memory (either
by reading it onto the heap or by memory-mapping it). Things that cannot be
memory-mapped (such as \fBstdin\fP) will be consumed until EOF before searching
can begin. In general, ripgrep will only do these things when necessary.
Specifically, if the \flag{multiline} flag is provided but the regex does
not contain patterns that would match \fB\\n\fP characters, then ripgrep
will automatically avoid reading each file into memory before searching it.
Nevertheless, if you only care about matches spanning at most one line, then it
is always better to disable multiline mode.
.sp
This overrides the \flag{stop-on-nonmatch} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline = v.unwrap_switch();
        if args.multiline {
            args.stop_on_nonmatch = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.multiline);

    let args = parse_low_raw(["--multiline"]).expect("Test parsing should succeed");
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U"]).expect("Test parsing should succeed");
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U", "--no-multiline"]).expect("Test parsing should succeed");
    assert_eq!(false, args.multiline);
}

/// --multiline-dotall
#[derive(Debug)]
pub(super) struct MultilineDotall;

impl Flag for MultilineDotall {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "multiline-dotall"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline-dotall")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Make '.' match line terminators."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enables "dot all" mode in all regex patterns. This causes \fB.\fP to
match line terminators when multiline searching is enabled. This flag has no
effect if multiline searching isn't enabled with the \flag{multiline} flag.
.sp
Normally, a \fB.\fP will match any character except line terminators. While
this behavior typically isn't relevant for line-oriented matching (since
matches can span at most one line), this can be useful when searching with the
\flag{multiline} flag. By default, multiline mode runs without "dot all" mode
enabled.
.sp
This flag is generally intended to be used in an alias or your ripgrep config
file if you prefer "dot all" semantics by default. Note that regardless of
whether this flag is used, "dot all" semantics can still be controlled via
inline flags in the regex pattern itself, e.g., \fB(?s:.)\fP always enables
"dot all" whereas \fB(?-s:.)\fP always disables "dot all". Moreover, you
can use character classes like \fB\\p{any}\fP to match any Unicode codepoint
regardless of whether "dot all" mode is enabled or not.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline_dotall = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline_dotall() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall"]).expect("Test parsing should succeed");
    assert_eq!(true, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall", "--no-multiline-dotall"])
        .expect("Test parsing should succeed");
    assert_eq!(false, args.multiline_dotall);
}

/// --null-data
#[derive(Debug)]
pub(super) struct NullData;

impl Flag for NullData {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "null-data"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use NUL as a line terminator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this flag causes ripgrep to use \fBNUL\fP as a line terminator instead
of the default of \fP\\n\fP.
.sp
This is useful when searching large binary files that would otherwise have
very long lines if \fB\\n\fP were used as the line terminator. In particular,
ripgrep requires that, at a minimum, each line must fit into memory. Using
\fBNUL\fP instead can be a useful stopgap to keep memory requirements low and
avoid OOM (out of memory) conditions.
.sp
This is also useful for processing NUL delimited data, such as that emitted
when using ripgrep's \flag{null} flag or \fBfind\fP's \fB\-\-print0\fP flag.
.sp
Using this flag implies \flag{text}. It also overrides \flag{crlf}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null-data has no negation");
        args.crlf = false;
        args.null_data = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null_data() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).expect("Test parsing should succeed");
    assert_eq!(false, args.null_data);
    assert_eq!(true, args.crlf);

    let args = parse_low_raw(["--crlf", "--null-data"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);
}

/// -x/--line-regexp
#[derive(Debug)]
pub(super) struct LineRegexp;

impl Flag for LineRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'x')
    }
    fn name_long(&self) -> &'static str {
        "line-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by line boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by line boundaries.
This is equivalent to surrounding every pattern with \fB^\fP and \fB$\fP. In
other words, this only prints lines where the entire line participates in a
match.
.sp
This overrides the \flag{word-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-regexp has no negation");
        args.boundary = Some(BoundaryMode::Line);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_regexp() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--line-regexp"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Line), args.boundary);

    let args = parse_low_raw(["-x"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}

/// -w/--word-regexp
#[derive(Debug)]
pub(super) struct WordRegexp;

impl Flag for WordRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'w')
    }
    fn name_long(&self) -> &'static str {
        "word-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by word boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by word boundaries.
This is equivalent to surrounding every pattern with \fB\\b{start-half}\fP
and \fB\\b{end-half}\fP.
.sp
This overrides the \flag{line-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--word-regexp has no negation");
        args.boundary = Some(BoundaryMode::Word);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_word_regexp() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--word-regexp"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-x", "-w"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w", "-x"]).expect("Test parsing should succeed");
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}

/// --stop-on-nonmatch
#[derive(Debug)]
pub(super) struct StopOnNonmatch;

impl Flag for StopOnNonmatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "stop-on-nonmatch"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Stop searching after a non-match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this option will cause ripgrep to stop reading a file once it
encounters a non-matching line after it has encountered a matching line.
This is useful if it is expected that all matches in a given file will be on
sequential lines, for example due to the lines being sorted.
.sp
This overrides the \flag{multiline} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--stop-on-nonmatch has no negation");
        args.stop_on_nonmatch = true;
        args.multiline = false;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_stop_on_nonmatch() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch"]).expect("Test parsing should succeed");
    assert_eq!(true, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch", "-U"]).expect("Test parsing should succeed");
    assert_eq!(true, args.multiline);
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["-U", "--stop-on-nonmatch"]).expect("Test parsing should succeed");
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);

    let args =
        parse_low_raw(["--stop-on-nonmatch", "--no-multiline"]).expect("Test parsing should succeed");
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);
}
