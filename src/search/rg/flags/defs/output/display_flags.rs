//! Display formatting output flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::LowArgs,
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// --byte-offset
#[derive(Debug)]
pub(in crate::search::rg::flags) struct ByteOffset;

impl Flag for ByteOffset {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'b')
    }
    fn name_long(&self) -> &'static str {
        "byte-offset"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-byte-offset")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Print the byte offset for each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print the 0-based byte offset within the input file before each line of output.
If \flag{only-matching} is specified, print the offset of the matched text
itself.
.sp
If ripgrep does transcoding, then the byte offset is in terms of the result
of transcoding and not the original data. This applies similarly to other
transformations on the data, such as decompression or a \flag{pre} filter.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.byte_offset = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_byte_offset() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--byte-offset"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["-b"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["--byte-offset", "--no-byte-offset"]).expect("Test parsing should succeed");
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--no-byte-offset", "-b"]).expect("Test parsing should succeed");
    assert_eq!(true, args.byte_offset);
}

/// --column
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Column;

impl Flag for Column {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "column"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-column")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show column numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show column numbers (1-based). This only shows the column numbers for the first
match on each line. This does not try to account for Unicode. One byte is equal
to one column. This implies \flag{line-number}.
.sp
When \flag{only-matching} is used, then the column numbers written correspond
to the start of each match.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.column = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_column() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.column);

    let args = parse_low_raw(["--column"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.column);

    let args = parse_low_raw(["--column", "--no-column"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.column);

    let args = parse_low_raw(["--no-column", "--column"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.column);
}

/// --heading
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Heading;

impl Flag for Heading {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "heading"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-heading")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print matches grouped by each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the file path above clusters of matches from each file instead
of printing the file path as a prefix for each matched line.
.sp
This is the default mode when printing to a tty.
.sp
When \fBstdout\fP is not a tty, then ripgrep will default to the standard
grep-like format. One can force this format in Unix-like environments by
piping the output of ripgrep to \fBcat\fP. For example, \fBrg\fP \fIfoo\fP \fB|
cat\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.heading = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_heading() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.heading);

    let args = parse_low_raw(["--heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.heading);

    let args = parse_low_raw(["--no-heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--heading", "--no-heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--no-heading", "--heading"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.heading);
}

/// -n/--line-number
#[derive(Debug)]
pub(in crate::search::rg::flags) struct LineNumber;

impl Flag for LineNumber {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'n')
    }
    fn name_long(&self) -> &'static str {
        "line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show line numbers (1-based).
.sp
This is enabled by default when stdout is connected to a tty.
.sp
This flag can be disabled by \flag{no-line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-number has no automatic negation");
        args.line_number = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_number() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n", "--no-line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);
}

/// -N/--no-line-number
#[derive(Debug)]
pub(in crate::search::rg::flags) struct LineNumberNo;

impl Flag for LineNumberNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'N')
    }
    fn name_long(&self) -> &'static str {
        "no-line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Suppress line numbers.
.sp
Line numbers are off by default when stdout is not connected to a tty.
.sp
Line numbers can be forcefully turned on by \flag{line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(
            v.unwrap_switch(),
            "--no-line-number has no automatic negation"
        );
        args.line_number = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_line_number() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--no-line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N", "--line-number"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.line_number);
}

/// --with-filename
#[derive(Debug)]
pub(in crate::search::rg::flags) struct WithFilename;

impl Flag for WithFilename {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'H')
    }
    fn name_long(&self) -> &'static str {
        "with-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print the file path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print the file path for each matching line.
This is the default when more than one file is searched. If \flag{heading} is
enabled (the default when printing to a tty), the file path will be shown above
clusters of matches from each file; otherwise, the file name will be shown as a
prefix for each matched line.
.sp
This flag overrides \flag{no-filename}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--with-filename has no defined negation");
        args.with_filename = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--with-filename"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);
}

/// --no-filename
#[derive(Debug)]
pub(in crate::search::rg::flags) struct WithFilenameNo;

impl Flag for WithFilenameNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'I')
    }
    fn name_long(&self) -> &'static str {
        "no-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Never print the path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to never print the file path with each matching
line. This is the default when ripgrep is explicitly instructed to search one
file or stdin.
.sp
This flag overrides \flag{with-filename}.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--no-filename has no defined negation");
        args.with_filename = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename_no() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--no-filename"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I", "-H"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H", "-I"]).expect("Test parsing should succeed");
    assert_eq!(Some(false), args.with_filename);
}
