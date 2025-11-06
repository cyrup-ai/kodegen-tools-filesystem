//! Case sensitivity and pattern matching flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{BoundaryMode, CaseMode, LowArgs},
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// -s/--case-sensitive
#[derive(Debug)]
pub(super) struct CaseSensitive;

impl Flag for CaseSensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b's')
    }
    fn name_long(&self) -> &'static str {
        "case-sensitive"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search case sensitively (default)."
    }
    fn doc_long(&self) -> &'static str {
        r"
Execute the search case sensitively. This is the default mode.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case insensitively by using inline
regex flags. For example, \fB(?i)abc\fP will match \fBabc\fP case insensitively
even when this flag is used.
.sp
This flag overrides the \flag{ignore-case} and \flag{smart-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Sensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_case_sensitive() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--case-sensitive"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);
}

/// -i/--ignore-case
#[derive(Debug)]
pub(super) struct IgnoreCase;

impl Flag for IgnoreCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'i')
    }
    fn name_long(&self) -> &'static str {
        "ignore-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Case insensitive search."
    }
    fn doc_long(&self) -> &'static str {
        r#"
When this flag is provided, all patterns will be searched case insensitively.
The case insensitivity rules used by ripgrep's default regex engine conform to
Unicode's "simple" case folding rules.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case sensitively by using
inline regex flags. For example, \fB(?\-i)abc\fP will match \fBabc\fP
case sensitively even when this flag is used.
.sp
This flag overrides \flag{case-sensitive} and \flag{smart-case}.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Insensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_case() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--ignore-case"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i", "-s"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s", "-i"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Insensitive, args.case);
}

/// -S/--smart-case
#[derive(Debug)]
pub(super) struct SmartCase;

impl Flag for SmartCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'S')
    }
    fn name_long(&self) -> &'static str {
        "smart-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Smart case search."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to searches case insensitively if the pattern is
all lowercase. Otherwise, ripgrep will search case sensitively.
.sp
A pattern is considered all lowercase if both of the following rules hold:
.sp
.IP \(bu 3n
First, the pattern contains at least one literal character. For example,
\fBa\\w\fP contains a literal (\fBa\fP) but just \fB\\w\fP does not.
.sp
.IP \(bu 3n
Second, of the literals in the pattern, none of them are considered to be
uppercase according to Unicode. For example, \fBfoo\\pL\fP has no uppercase
literals but \fBFoo\\pL\fP does.
.PP
This overrides the \flag{case-sensitive} and \flag{ignore-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--smart-case flag has no negation");
        args.case = CaseMode::Smart;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_smart_case() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--smart-case"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S", "-s"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-S", "-i"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-s", "-S"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-i", "-S"]).expect("Test parsing should succeed");
    assert_eq!(CaseMode::Smart, args.case);
}

/// -F/--fixed-strings
#[derive(Debug)]
pub(super) struct FixedStrings;

impl Flag for FixedStrings {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'F')
    }
    fn name_long(&self) -> &'static str {
        "fixed-strings"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-fixed-strings")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Treat all patterns as literals."
    }
    fn doc_long(&self) -> &'static str {
        r"
Treat all patterns as literals instead of as regular expressions. When this
flag is used, special regular expression meta characters such as \fB.(){}*+\fP
should not need be escaped.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.fixed_strings = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_fixed_strings() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--fixed-strings"]).expect("Test parsing should succeed");
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F"]).expect("Test parsing should succeed");
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F", "--no-fixed-strings"]).expect("Test parsing should succeed");
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--no-fixed-strings", "-F"]).expect("Test parsing should succeed");
    assert_eq!(true, args.fixed_strings);
}

/// -v/--invert-match
#[derive(Debug)]
pub(super) struct InvertMatch;

impl Flag for InvertMatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'v')
    }
    fn name_long(&self) -> &'static str {
        "invert-match"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-invert-match")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Invert matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag inverts matching. That is, instead of printing lines that match,
ripgrep will print lines that don't match.
.sp
Note that this only inverts line-by-line matching. For example, combining this
flag with \flag{files-with-matches} will emit files that contain any lines
that do not match the patterns given. That's not the same as, for example,
\flag{files-without-match}, which will emit files that do not contain any
matching lines.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.invert_match = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_invert_match() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.invert_match);

    let args = parse_low_raw(["--invert-match"]).expect("Test parsing should succeed");
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v"]).expect("Test parsing should succeed");
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v", "--no-invert-match"]).expect("Test parsing should succeed");
    assert_eq!(false, args.invert_match);
}
