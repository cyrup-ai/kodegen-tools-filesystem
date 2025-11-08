//! Tests for OtherBehaviors category flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use kodegen_tools_filesystem::search::rg::flags::{Mode, SearchMode};

#[test]
fn test_files() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--files"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Files, args.mode);
}

#[test]
fn test_no_config() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_config);

    let args = parse_low_raw(["--no-config"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_config);
}
