//! Limiting and filtering output flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::LowArgs,
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::super::convert;

/// --include-zero
#[derive(Debug)]
pub(in crate::search::rg::flags) struct IncludeZero;

impl Flag for IncludeZero {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "include-zero"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-include-zero")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Include zero matches in summary output."
    }
    fn doc_long(&self) -> &'static str {
        r"
When used with \flag{count} or \flag{count-matches}, this causes ripgrep to
print the number of matches for each file even if there were zero matches. This
is disabled by default but can be enabled to make ripgrep behave more like
grep.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.include_zero = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_include_zero() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);

    let args = parse_low_raw(["--include-zero"]).expect("Test parsing should succeed");
    assert_eq!(true, args.include_zero);

    let args = parse_low_raw(["--include-zero", "--no-include-zero"]).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);
}

/// -M/--max-columns
#[derive(Debug)]
pub(in crate::search::rg::flags) struct MaxColumns;

impl Flag for MaxColumns {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'M')
    }
    fn name_long(&self) -> &'static str {
        "max-columns"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Omit lines longer than this limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
When given, ripgrep will omit lines longer than this limit in bytes. Instead of
printing long lines, only the number of matches in that line is printed.
.sp
When this flag is omitted or is set to \fB0\fP, then it has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let max = convert::u64(&v.unwrap_value())?;
        args.max_columns = if max == 0 { None } else { Some(max) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.max_columns);

    let args = parse_low_raw(["--max-columns", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M", "5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M5"]).expect("Test parsing should succeed");
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["--max-columns", "5", "-M0"]).expect("Test parsing should succeed");
    assert_eq!(None, args.max_columns);
}

/// --max-columns-preview
#[derive(Debug)]
pub(in crate::search::rg::flags) struct MaxColumnsPreview;

impl Flag for MaxColumnsPreview {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "max-columns-preview"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-max-columns-preview")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show preview for lines exceeding the limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
Prints a preview for lines exceeding the configured max column limit.
.sp
When the \flag{max-columns} flag is used, ripgrep will by default completely
replace any line that is too long with a message indicating that a matching
line was removed. When this flag is combined with \flag{max-columns}, a preview
of the line (corresponding to the limit size) is shown instead, where the part
of the line exceeding the limit is not shown.
.sp
If the \flag{max-columns} flag is not set, then this has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_columns_preview = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns_preview() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.max_columns_preview);

    let args = parse_low_raw(["--max-columns-preview"]).expect("Test parsing should succeed");
    assert_eq!(true, args.max_columns_preview);

    let args =
        parse_low_raw(["--max-columns-preview", "--no-max-columns-preview"])
            .expect("Test parsing should succeed");
    assert_eq!(false, args.max_columns_preview);
}
