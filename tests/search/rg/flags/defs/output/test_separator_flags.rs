//! Tests for separator-related output flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use bstr::BString;

#[test]
fn test_field_context_separator() {
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

#[test]
fn test_field_match_separator() {
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
