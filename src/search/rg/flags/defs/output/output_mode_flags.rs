//! Output mode control flags.

use bstr::BString;

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::LowArgs,
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::super::convert;

/// -0/--null
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Null;

impl Flag for Null {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'0')
    }
    fn name_long(&self) -> &'static str {
        "null"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print a NUL byte after file paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Whenever a file path is printed, follow it with a \fBNUL\fP byte. This includes
printing file paths before matches, and when printing a list of matching files
such as with \flag{count}, \flag{files-with-matches} and \flag{files}. This
option is useful for use with \fBxargs\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null has no negation");
        args.null = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.null);

    let args = parse_low_raw(["--null"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);

    let args = parse_low_raw(["-0"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);
}

/// -o/--only-matching
#[derive(Debug)]
pub(in crate::search::rg::flags) struct OnlyMatching;

impl Flag for OnlyMatching {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'o')
    }
    fn name_long(&self) -> &'static str {
        "only-matching"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print only matched parts of a line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print only the matched (non-empty) parts of a matching line, with each such
part on a separate output line.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--only-matching does not have a negation");
        args.only_matching = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_only_matching() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.only_matching);

    let args = parse_low_raw(["--only-matching"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);

    let args = parse_low_raw(["-o"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);
}

/// -q/--quiet
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Quiet;

impl Flag for Quiet {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'q')
    }
    fn name_long(&self) -> &'static str {
        "quiet"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Do not print anything to stdout."
    }
    fn doc_long(&self) -> &'static str {
        r"
Do not print anything to stdout. If a match is found in a file, then ripgrep
will stop searching. This is useful when ripgrep is used only for its exit code
(which will be an error code if no matches are found).
.sp
When \flag{files} is used, ripgrep will stop finding files after finding the
first file that does not match any ignore rules.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--quiet has no negation");
        args.quiet = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_quiet() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.quiet);

    let args = parse_low_raw(["--quiet"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    // flags like -l and --json cannot override -q, regardless of order
    let args = parse_low_raw(["-q", "--json"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(true, args.quiet);
}

/// -r/--replace
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Replace;

impl Flag for Replace {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'r')
    }
    fn name_long(&self) -> &'static str {
        "replace"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("REPLACEMENT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Replace matches with the given text."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Replaces every match with the text given when printing results. Neither this
flag nor any other ripgrep flag will modify your files.
.sp
Capture group indices (e.g., \fB$\fP\fI5\fP) and names (e.g., \fB$\fP\fIfoo\fP)
are supported in the replacement string. Capture group indices are numbered
based on the position of the opening parenthesis of the group, where the
leftmost such group is \fB$\fP\fI1\fP. The special \fB$\fP\fI0\fP group
corresponds to the entire match.
.sp
The name of a group is formed by taking the longest string of letters, numbers
and underscores (i.e. \fB[_0-9A-Za-z]\fP) after the \fB$\fP. For example,
\fB$\fP\fI1a\fP will be replaced with the group named \fI1a\fP, not the
group at index \fI1\fP. If the group's name contains characters that aren't
letters, numbers or underscores, or you want to immediately follow the group
with another string, the name should be put inside braces. For example,
\fB${\fP\fI1\fP\fB}\fP\fIa\fP will take the content of the group at index
\fI1\fP and append \fIa\fP to the end of it.
.sp
If an index or name does not refer to a valid capture group, it will be
replaced with an empty string.
.sp
In shells such as Bash and zsh, you should wrap the pattern in single quotes
instead of double quotes. Otherwise, capture group indices will be replaced by
expanded shell variables which will most likely be empty.
.sp
To write a literal \fB$\fP, use \fB$$\fP.
.sp
Note that the replacement by default replaces each match, and not the entire
line. To replace the entire line, you should match the entire line.
.sp
This flag can be used with the \flag{only-matching} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.replace = Some(convert::string(v.unwrap_value())?.into());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_replace() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.replace);

    let args = parse_low_raw(["--replace", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["--replace", "-foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("-foo")), args.replace);

    let args = parse_low_raw(["-r", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-rbar"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("bar")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-r", ""]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("")), args.replace);
}

/// --trim
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Trim;

impl Flag for Trim {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "trim"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-trim")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Trim prefix whitespace from matches."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, all ASCII whitespace at the beginning of each line printed will be
removed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.trim = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_trim() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);

    let args = parse_low_raw(["--trim"]).expect("Test parsing should succeed");
    assert_eq!(true, args.trim);

    let args = parse_low_raw(["--trim", "--no-trim"]).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);
}

/// --vimgrep
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Vimgrep;

impl Flag for Vimgrep {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "vimgrep"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print results in a vim compatible format."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print results with every match on its own line,
including line numbers and column numbers.
.sp
With this option, a line with more than one match will be printed in its
entirety more than once. For that reason, the total amount of output as a
result of this flag can be quadratic in the size of the input. For example,
if the pattern matches every byte in an input file, then each line will be
repeated for every byte matched. For this reason, users should only use this
flag when there is no other choice. Editor integrations should prefer some
other way of reading results from ripgrep, such as via the \flag{json} flag.
One alternative to avoiding exorbitant memory usage is to force ripgrep into
single threaded mode with the \flag{threads} flag. Note though that this will
not impact the total size of the output, just the heap memory that ripgrep will
use.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--vimgrep has no negation");
        args.vimgrep = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_vimgrep() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.vimgrep);

    let args = parse_low_raw(["--vimgrep"]).expect("Test parsing should succeed");
    assert_eq!(true, args.vimgrep);
}
