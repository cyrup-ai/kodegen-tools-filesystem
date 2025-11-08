//! Tests for display formatting output flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;

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

#[test]
fn test_with_filename() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--with-filename"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H"]).expect("Test parsing should succeed");
    assert_eq!(Some(true), args.with_filename);
}

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
