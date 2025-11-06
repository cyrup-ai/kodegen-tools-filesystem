//! Separator-related output flags.

use {anyhow::Context as AnyhowContext, bstr::ByteVec};

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::LowArgs,
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::super::convert;

/// --field-context-separator
#[derive(Debug)]
pub(in crate::search::rg::flags) struct FieldContextSeparator;

impl Flag for FieldContextSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-context-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field context separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field context separator. This separator is only used when printing
contextual lines. It is used to delimit file paths, line numbers, columns and
the contextual line itself. The separator may be any number of bytes, including
zero. Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB-\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::search::rg::flags::lowargs::FieldContextSeparator as Separator;

        args.field_context_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_context_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BString::from("-"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator=XYZ"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw([
        "--field-context-separator",
        "XYZ",
        "--field-context-separator",
        "ABC",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        BString::from("ABC"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator", r"\t"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\t"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from("\x00"),
        args.field_context_separator.into_bytes()
    );

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-context-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_context_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --field-match-separator
#[derive(Debug)]
pub(in crate::search::rg::flags) struct FieldMatchSeparator;

impl Flag for FieldMatchSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-match-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field match separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field match separator. This separator is only used when printing
matching lines. It is used to delimit file paths, line numbers, columns and the
matching line itself. The separator may be any number of bytes, including zero.
Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB:\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::search::rg::flags::lowargs::FieldMatchSeparator as Separator;

        args.field_match_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_match_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(BString::from(":"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator=XYZ"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw([
        "--field-match-separator",
        "XYZ",
        "--field-match-separator",
        "ABC",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(BString::from("ABC"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\t"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\t"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(BString::from("\x00"), args.field_match_separator.into_bytes());

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-match-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_match_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-match-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --path-separator
#[derive(Debug)]
pub(in crate::search::rg::flags) struct PathSeparator;

impl Flag for PathSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "path-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the path separator for printing paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the path separator to use when printing file paths. This defaults to your
platform's path separator, which is \fB/\fP on Unix and \fB\\\fP on Windows.
This flag is intended for overriding the default when the environment demands
it (e.g., cygwin). A path separator is limited to a single byte.
.sp
Setting this flag to an empty string reverts it to its default behavior. That
is, the path separator is automatically chosen based on the environment.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let s = convert::string(v.unwrap_value())?;
        let raw = Vec::unescape_bytes(&s);
        args.path_separator = if raw.is_empty() {
            None
        } else if raw.len() == 1 {
            Some(raw[0])
        } else {
            anyhow::bail!(
                "A path separator must be exactly one byte, but \
                 the given separator is {len} bytes: {sep}\n\
                 In some shells on Windows '/' is automatically \
                 expanded. Use '//' instead.",
                len = raw.len(),
                sep = s,
            )
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_path_separator() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.path_separator);

    let args = parse_low_raw(["--path-separator", "/"]).expect("Test parsing should succeed");
    assert_eq!(Some(b'/'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\"]).expect("Test parsing should succeed");
    assert_eq!(Some(b'\\'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\x00"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\x00"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\0"]).expect("Test parsing should succeed");
    assert_eq!(Some(0), args.path_separator);

    let args =
        parse_low_raw(["--path-separator", r"\x00", "--path-separator=/"])
            .expect("Test parsing should succeed");
    assert_eq!(Some(b'/'), args.path_separator);

    let result = parse_low_raw(["--path-separator", "foo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--path-separator", r"\\x00"]);
    assert!(result.is_err(), "{result:?}");
}
