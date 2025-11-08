//! Tests for context-related output flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use kodegen_tools_filesystem::search::rg::flags::ContextMode;
use bstr::BString;

#[test]
fn test_after_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_after(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--after-context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--after-context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5", "-A10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-A5", "-A0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-A5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-A5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--after-context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--after-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

#[test]
fn test_before_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_before(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--before-context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--before-context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5", "-B10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-B5", "-B0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-B5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-B5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--before-context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--before-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

#[test]
fn test_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_both(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--context", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--context=5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C", "5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5", "-C10"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-C5", "-C0"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-C5", "--passthru"]).expect("Test parsing should succeed");
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-C5"]).expect("Test parsing should succeed");
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--context", n.as_str()]).expect("Test parsing should succeed");
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }

    // Test the interaction between -A/-B and -C. Basically, -A/-B always
    // partially overrides -C, regardless of where they appear relative to
    // each other. This behavior is also how GNU grep works, and it also makes
    // logical sense to me: -A/-B are the more specific flags.
    let args = parse_low_raw(["-A1", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-B1", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-A1", "-B2", "-C5"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());

    // These next three are like the ones above, but with -C before -A/-B. This
    // tests that -A and -B only partially override -C. That is, -C1 -A2 is
    // equivalent to -B1 -A2.
    let args = parse_low_raw(["-C5", "-A1"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-B1"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-A1", "-B2"]).expect("Test parsing should succeed");
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());
}

#[test]
fn test_context_separator() {
    use kodegen_tools_filesystem::search::rg::flags::ContextSeparator as Separator;

    let getbytes = |ctxsep: Separator| ctxsep.into_bytes().map(BString::from);

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("--")), getbytes(args.context_separator));

    let args = parse_low_raw(["--context-separator", "XYZ"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    let args = parse_low_raw(["--no-context-separator"]).expect("Test parsing should succeed");
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--context-separator",
        "XYZ",
        "--no-context-separator",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--no-context-separator",
        "--context-separator",
        "XYZ",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--context-separator", r"\xFF"]).expect("Test parsing should succeed");
    assert_eq!(Some(BString::from(b"\xFF")), getbytes(args.context_separator));

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
            OsStr::from_bytes(b"--context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}
