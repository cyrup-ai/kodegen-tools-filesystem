//! Tests for limiting and filtering output flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;

#[test]
fn test_include_zero() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);

    let args = parse_low_raw(["--include-zero"]).expect("Test parsing should succeed");
    assert_eq!(true, args.include_zero);

    let args = parse_low_raw(["--include-zero", "--no-include-zero"]).expect("Test parsing should succeed");
    assert_eq!(false, args.include_zero);
}

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
