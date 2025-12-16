//! Regression tests for content search regex validation and auto-fallback
//!
//! Tests the behavior where invalid regex patterns are automatically detected
//! and fallen back to literal search when pattern type is INFERRED.

use kodegen_mcp_schema::filesystem::{
    BinaryMode, CaseMode, EngineChoice as Engine, ReturnMode, SearchIn,
};
use kodegen_tools_filesystem::search::{
    manager::{content_search::execute, context::SearchContext},
    types::{PatternMode, SearchSessionOptions},
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test directory with sample HTML files
fn setup_test_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.html");

    let content = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test Page (with paren)</title>
</head>
<body>
    <a href="">Empty href</a>
    <a href="https://example.com">Example link</a>
    <a href="/path/to/page">Internal link</a>
</body>
</html>
"#;

    fs::write(&test_file, content).unwrap();
    temp_dir
}

#[test]
fn test_auto_fallback_unescaped_quotes() {
    // Pattern with unmatched parenthesis - not a valid regex
    // When type is INFERRED, should auto-fallback to literal/Substring mode

    let temp_dir = setup_test_dir();
    let root = PathBuf::from(temp_dir.path());

    let options = SearchSessionOptions {
        root_path: temp_dir.path().to_string_lossy().to_string(),
        pattern: "(".to_string(),
        search_in: SearchIn::Content,
        pattern_mode: None, // INFERRED type
        literal_search: false,
        case_mode: CaseMode::Sensitive,
        return_only: ReturnMode::Matches,
        max_results: Some(100),
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        include_hidden: false,
        no_ignore: false,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        boundary_mode: None,
        invert_match: false,
        engine: Engine::Auto,
        preprocessor: None,
        preprocessor_globs: vec![],
        search_zip: false,
        binary_mode: BinaryMode::Auto,
        multiline: false,
        max_filesize: None,
        max_depth: None,
        only_matching: false,
        sort_by: None,
        sort_direction: None,
        encoding: None,
    };

    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);

    // Execute search
    execute(&options, &root, &mut ctx);

    // Context should show pattern type was changed to Substring
    assert_eq!(
        ctx.pattern_type,
        Some(PatternMode::Substring),
        "Pattern type should have fallen back to Substring"
    );

    // Should find matches (literal search for href="")
    let results = ctx.results().blocking_read();
    assert!(
        !results.is_empty(),
        "Should find matches with literal search"
    );

    // Should be marked as complete
    assert!(ctx.is_complete, "Search should complete successfully");
}

#[test]
fn test_explicit_regex_mode_not_overridden() {
    // When user EXPLICITLY sets pattern_mode=Regex, respect their choice
    // The pattern ( is not a valid regex, so it will error, but mode is not overridden

    let temp_dir = setup_test_dir();
    let root = PathBuf::from(temp_dir.path());

    let options = SearchSessionOptions {
        root_path: temp_dir.path().to_string_lossy().to_string(),
        pattern: "(".to_string(),
        search_in: SearchIn::Content,
        pattern_mode: Some(PatternMode::Regex), // EXPLICIT regex mode
        literal_search: false,
        case_mode: CaseMode::Sensitive,
        return_only: ReturnMode::Matches,
        max_results: Some(100),
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        include_hidden: false,
        no_ignore: false,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        boundary_mode: None,
        invert_match: false,
        engine: Engine::Auto,
        preprocessor: None,
        preprocessor_globs: vec![],
        search_zip: false,
        binary_mode: BinaryMode::Auto,
        multiline: false,
        max_filesize: None,
        max_depth: None,
        only_matching: false,
        sort_by: None,
        sort_direction: None,
        encoding: None,
    };

    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);

    // Execute search
    execute(&options, &root, &mut ctx);

    // Context should show Regex type (explicit choice respected)
    assert_eq!(
        ctx.pattern_type,
        Some(PatternMode::Regex),
        "Explicit pattern_mode should not be overridden"
    );
}

#[test]
fn test_valid_regex_not_affected() {
    // Valid regex patterns should work normally
    // Pattern href=".+" is a valid regex

    let temp_dir = setup_test_dir();
    let root = PathBuf::from(temp_dir.path());

    let options = SearchSessionOptions {
        root_path: temp_dir.path().to_string_lossy().to_string(),
        pattern: r#"href=".+""#.to_string(),
        search_in: SearchIn::Content,
        pattern_mode: None, // INFERRED type
        literal_search: false,
        case_mode: CaseMode::Sensitive,
        return_only: ReturnMode::Matches,
        max_results: Some(100),
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        include_hidden: false,
        no_ignore: false,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        boundary_mode: None,
        invert_match: false,
        engine: Engine::Auto,
        preprocessor: None,
        preprocessor_globs: vec![],
        search_zip: false,
        binary_mode: BinaryMode::Auto,
        multiline: false,
        max_filesize: None,
        max_depth: None,
        only_matching: false,
        sort_by: None,
        sort_direction: None,
        encoding: None,
    };

    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);

    // Execute search
    execute(&options, &root, &mut ctx);

    // Context should show Regex type (no fallback needed)
    assert_eq!(
        ctx.pattern_type,
        Some(PatternMode::Regex),
        "Valid regex should remain as Regex type"
    );

    // Should find matches
    let results = ctx.results().blocking_read();
    assert!(
        !results.is_empty(),
        "Valid regex should find matches"
    );

    // Should be marked as complete
    assert!(ctx.is_complete, "Search should complete successfully");
}

#[test]
fn test_escaped_quotes_work_as_regex() {
    // Properly escaped pattern href=\"\" should work as regex

    let temp_dir = setup_test_dir();
    let root = PathBuf::from(temp_dir.path());

    let options = SearchSessionOptions {
        root_path: temp_dir.path().to_string_lossy().to_string(),
        pattern: r#"href=\"\""#.to_string(), // Escaped quotes
        search_in: SearchIn::Content,
        pattern_mode: None, // INFERRED type
        literal_search: false,
        case_mode: CaseMode::Sensitive,
        return_only: ReturnMode::Matches,
        max_results: Some(100),
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        include_hidden: false,
        no_ignore: false,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        boundary_mode: None,
        invert_match: false,
        engine: Engine::Auto,
        preprocessor: None,
        preprocessor_globs: vec![],
        search_zip: false,
        binary_mode: BinaryMode::Auto,
        multiline: false,
        max_filesize: None,
        max_depth: None,
        only_matching: false,
        sort_by: None,
        sort_direction: None,
        encoding: None,
    };

    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);

    // Execute search
    execute(&options, &root, &mut ctx);

    // Context should show Regex type (valid regex)
    assert_eq!(
        ctx.pattern_type,
        Some(PatternMode::Regex),
        "Escaped pattern should remain as Regex type"
    );

    // Should find matches for empty href attributes
    let results = ctx.results().blocking_read();
    assert!(
        !results.is_empty(),
        "Escaped pattern should find empty href attributes"
    );

    // Should be marked as complete
    assert!(ctx.is_complete, "Search should complete successfully");
}
