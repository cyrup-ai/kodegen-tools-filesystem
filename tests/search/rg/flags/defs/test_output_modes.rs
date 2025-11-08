//! Tests for OutputModes category flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use kodegen_tools_filesystem::search::rg::flags::{Mode, SearchMode};

#[test]
fn test_count() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);
}

#[test]
fn test_count_matches() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count-matches"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);
}

#[test]
fn test_json() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Json), args.mode);

    let args = parse_low_raw(["--json", "--no-json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json", "--files", "--no-json"]).expect("Test parsing should succeed");
    assert_eq!(Mode::Files, args.mode);
}
