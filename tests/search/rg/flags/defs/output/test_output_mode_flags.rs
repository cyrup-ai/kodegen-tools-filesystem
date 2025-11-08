//! Tests for output mode flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use bstr::BString;

#[test]
fn test_null() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.null);

    let args = parse_low_raw(["--null"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);

    let args = parse_low_raw(["-0"]).expect("Test parsing should succeed");
    assert_eq!(true, args.null);
}

#[test]
fn test_only_matching() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.only_matching);

    let args = parse_low_raw(["--only-matching"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);

    let args = parse_low_raw(["-o"]).expect("Test parsing should succeed");
    assert_eq!(true, args.only_matching);
}

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

#[test]
fn test_trim() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);

    let args = parse_low_raw(["--trim"]).expect("Test parsing should succeed");
    assert_eq!(true, args.trim);

    let args = parse_low_raw(["--trim", "--no-trim"]).expect("Test parsing should succeed");
    assert_eq!(false, args.trim);
}

#[test]
fn test_vimgrep() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.vimgrep);

    let args = parse_low_raw(["--vimgrep"]).expect("Test parsing should succeed");
    assert_eq!(true, args.vimgrep);
}
