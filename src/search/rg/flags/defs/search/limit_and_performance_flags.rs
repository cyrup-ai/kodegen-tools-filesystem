//! Limit, performance, and text handling flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{BinaryMode, MmapMode, LowArgs},
};

use super::super::convert;

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// --dfa-size-limit
#[derive(Debug)]
pub(super) struct DfaSizeLimit;

impl Flag for DfaSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "dfa-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The upper size limit of the regex DFA."
    }
    fn doc_long(&self) -> &'static str {
        r"
The upper size limit of the regex DFA. The default limit is something generous
for any single pattern or for many smallish patterns. This should only be
changed on very large regex inputs where the (slower) fallback regex engine may
otherwise be used if the limit is reached.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.dfa_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_dfa_size_limit() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.dfa_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--dfa-size-limit", "9G"]).expect("Test parsing should succeed");
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args = parse_low_raw(["--dfa-size-limit=9G"]).expect("Test parsing should succeed");
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args =
            parse_low_raw(["--dfa-size-limit=9G", "--dfa-size-limit=0"])
                .expect("Test parsing should succeed");
        assert_eq!(Some(0), args.dfa_size_limit);
    }

    let args = parse_low_raw(["--dfa-size-limit=0K"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0M"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0G"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.dfa_size_limit);

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// -m/--max-count
#[derive(Debug)]
pub(super) struct MaxCount;

impl Flag for MaxCount {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'm')
    }
    fn name_long(&self) -> &'static str {
        "max-count"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Limit the number of matching lines."
    }
    fn doc_long(&self) -> &'static str {
        r"
Limit the number of matching lines per file searched to \fINUM\fP.
.sp
When \flag{multiline} is used, a single match that spans multiple lines is only
counted once for the purposes of this limit. Multiple matches in a single line
are counted only once, as they would be in non-multiline mode.
.sp
When combined with \flag{after-context} or \flag{context}, it's possible for
more matches than the maximum to be printed if contextual lines contain a
match.
.sp
Note that \fB0\fP is a legal value but not likely to be useful. When used,
ripgrep won't search anything.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_count = Some(convert::u64(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_count() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.max_count);

    let args = parse_low_raw(["--max-count", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5", "--max-count=10"]).expect("Test parsing should succeed");
    assert_eq!(Some(10), args.max_count);
    let args = parse_low_raw(["-m0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.max_count);
}

/// --regex-size-limit
#[derive(Debug)]
pub(super) struct RegexSizeLimit;

impl Flag for RegexSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "regex-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The size limit of the compiled regex."
    }
    fn doc_long(&self) -> &'static str {
        r"
The size limit of the compiled regex, where the compiled regex generally
corresponds to a single object in memory that can match all of the patterns
provided to ripgrep. The default limit is generous enough that most reasonable
patterns (or even a small number of them) should fit.
.sp
This useful to change when you explicitly want to let ripgrep spend potentially
much more time and/or memory building a regex matcher.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.regex_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_regex_size_limit() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.regex_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--regex-size-limit", "9G"]).expect("Test parsing should succeed");
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args = parse_low_raw(["--regex-size-limit=9G"]).expect("Test parsing should succeed");
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args =
            parse_low_raw(["--regex-size-limit=9G", "--regex-size-limit=0"])
                .expect("Test parsing should succeed");
        assert_eq!(Some(0), args.regex_size_limit);
    }

    let args = parse_low_raw(["--regex-size-limit=0K"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0M"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0G"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.regex_size_limit);

    let result =
        parse_low_raw(["--regex-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--regex-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// --mmap
#[derive(Debug)]
pub(super) struct Mmap;

impl Flag for Mmap {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "mmap"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-mmap")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search with memory maps when possible."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will search using memory maps when possible. This is
enabled by default when ripgrep thinks it will be faster.
.sp
Memory map searching cannot be used in all circumstances. For example, when
searching virtual files or streams likes \fBstdin\fP. In such cases, memory
maps will not be used even when this flag is enabled.
.sp
Note that ripgrep may abort unexpectedly when memory maps are used if it
searches a file that is simultaneously truncated. Users can opt out of this
possibility by disabling memory maps.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.mmap = if v.unwrap_switch() {
            MmapMode::AlwaysTryMmap
        } else {
            MmapMode::Never
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_mmap() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(MmapMode::Auto, args.mmap);

    let args = parse_low_raw(["--mmap"]).expect("Test parsing should succeed");
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);

    let args = parse_low_raw(["--no-mmap"]).expect("Test parsing should succeed");
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--mmap", "--no-mmap"]).expect("Test parsing should succeed");
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--no-mmap", "--mmap"]).expect("Test parsing should succeed");
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);
}

/// -j/--threads
#[derive(Debug)]
pub(super) struct Threads;

impl Flag for Threads {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'j')
    }
    fn name_long(&self) -> &'static str {
        "threads"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Set the approximate number of threads to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag sets the approximate number of threads to use. A value of \fB0\fP
(which is the default) causes ripgrep to choose the thread count using
heuristics.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let threads = convert::usize(&v.unwrap_value())?;
        args.threads = if threads == 0 { None } else { Some(threads) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_threads() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.threads);

    let args = parse_low_raw(["--threads", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5", "-j10"]).expect("Test parsing should succeed");
    assert_eq!(Some(10), args.threads);

    let args = parse_low_raw(["-j5", "-j0"]).expect("Test parsing should succeed");
    assert_eq!(None, args.threads);
}

/// -a/--text
#[derive(Debug)]
pub(super) struct Text;

impl Flag for Text {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'a')
    }
    fn name_long(&self) -> &'static str {
        "text"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-text")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search binary files as if they were text."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to search binary files as if they were text. When
this flag is present, ripgrep's binary file detection is disabled. This means
that when a binary file is searched, its contents may be printed if there is
a match. This may cause escape codes to be printed that alter the behavior of
your terminal.
.sp
When binary file detection is enabled, it is imperfect. In general, it uses
a simple heuristic. If a \fBNUL\fP byte is seen during search, then the file
is considered binary and searching stops (unless this flag is present).
Alternatively, if the \flag{binary} flag is used, then ripgrep will only quit
when it sees a \fBNUL\fP byte after it sees a match (or searches the entire
file).
.sp
This flag overrides the \flag{binary} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.binary = if v.unwrap_switch() {
            BinaryMode::AsText
        } else {
            BinaryMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_text() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--text"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-text"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["-a", "--binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "-a"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-binary"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--binary", "--no-text"]).expect("Test parsing should succeed");
    assert_eq!(BinaryMode::Auto, args.binary);
}
