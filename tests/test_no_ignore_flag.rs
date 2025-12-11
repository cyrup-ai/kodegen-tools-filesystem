//! Integration tests for no_ignore flag behavior
//!
//! Tests that `no_ignore: true` correctly bypasses .gitignore rules
//! for both file_search and content_search.

use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use tempfile::TempDir;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
            .is_test(true)
            .init();
    });
}

use kodegen_tools_filesystem::search::manager::context::SearchContext;
use kodegen_tools_filesystem::search::manager::file_search;
use kodegen_tools_filesystem::search::types::{
    BinaryMode, CaseMode, Engine, ReturnMode, SearchIn, SearchSessionOptions,
};

/// Helper to create SearchSessionOptions with common defaults
fn make_options(root_path: &str, pattern: &str, no_ignore: bool) -> SearchSessionOptions {
    SearchSessionOptions {
        root_path: root_path.to_string(),
        pattern: pattern.to_string(),
        search_in: SearchIn::Filenames,
        file_pattern: None,
        r#type: vec![],
        type_not: vec![],
        case_mode: CaseMode::Sensitive,
        max_results: Some(100),
        include_hidden: false,
        no_ignore,
        context: 0,
        before_context: None,
        after_context: None,
        timeout_ms: None,
        early_termination: None,
        literal_search: false,
        pattern_mode: None,
        boundary_mode: None,
        return_only: ReturnMode::Matches,
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
    }
}

/// Test: no_ignore=false should respect .gitignore (files excluded)
#[test]
fn test_gitignore_respected_when_no_ignore_false() {
    // Create temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");

    // Create test files
    fs::write(temp_path.join("visible.md"), "# Visible").expect("Failed to write visible.md");
    fs::write(temp_path.join("hidden.md"), "# Hidden").expect("Failed to write hidden.md");

    // Create .gitignore that ignores hidden.md
    fs::write(temp_path.join(".gitignore"), "hidden.md\n").expect("Failed to write .gitignore");

    // Create options with no_ignore=false (respect gitignore)
    let options = make_options(&temp_path.to_string_lossy(), ".*\\.md$", false);

    // Create context and execute
    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);
    let root = PathBuf::from(temp_path);
    file_search::execute(&options, &root, &mut ctx);

    // Verify results
    let results = ctx.results().blocking_read();

    println!("Test: no_ignore=false");
    println!("  Found {} files:", results.len());
    for r in results.iter() {
        println!("    - {}", r.file);
    }

    // Should only find visible.md (hidden.md excluded by .gitignore)
    assert_eq!(results.len(), 1, "Should find only 1 file when .gitignore is respected");
    assert!(
        results[0].file.contains("visible.md"),
        "Should find visible.md, found: {}",
        results[0].file
    );
}

/// Test: no_ignore=true should bypass .gitignore (all files found)
#[test]
fn test_gitignore_bypassed_when_no_ignore_true() {
    // Create temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");

    // Create test files
    fs::write(temp_path.join("visible.md"), "# Visible").expect("Failed to write visible.md");
    fs::write(temp_path.join("hidden.md"), "# Hidden").expect("Failed to write hidden.md");

    // Create .gitignore that ignores hidden.md
    fs::write(temp_path.join(".gitignore"), "hidden.md\n").expect("Failed to write .gitignore");

    // Create options with no_ignore=true (bypass gitignore)
    let options = make_options(&temp_path.to_string_lossy(), ".*\\.md$", true);

    // Create context and execute
    let mut ctx = SearchContext::new(100, ReturnMode::Matches, None);
    let root = PathBuf::from(temp_path);
    file_search::execute(&options, &root, &mut ctx);

    // Verify results
    let results = ctx.results().blocking_read();

    println!("Test: no_ignore=true");
    println!("  Found {} files:", results.len());
    for r in results.iter() {
        println!("    - {}", r.file);
    }

    // Should find BOTH files (hidden.md NOT excluded)
    assert_eq!(
        results.len(),
        2,
        "Should find 2 files when .gitignore is bypassed, found: {:?}",
        results.iter().map(|r| &r.file).collect::<Vec<_>>()
    );
}

/// Test: no_ignore=true with aggressive .gitignore (ignores everything with *)
#[test]
fn test_no_ignore_with_aggressive_gitignore() {
    init_logging();

    // Create temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Create .git directory to make it a git repository
    fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");

    // Create test files
    fs::write(temp_path.join("file1.md"), "# File 1").expect("Failed to write file1.md");
    fs::write(temp_path.join("file2.md"), "# File 2").expect("Failed to write file2.md");
    fs::write(temp_path.join("file3.md"), "# File 3").expect("Failed to write file3.md");

    // Create aggressive .gitignore that ignores EVERYTHING except itself
    fs::write(temp_path.join(".gitignore"), "*\n!.gitignore\n").expect("Failed to write .gitignore");

    // === Test with no_ignore=false ===
    let options_ignore = make_options(&temp_path.to_string_lossy(), ".*\\.md$", false);
    let mut ctx_ignore = SearchContext::new(100, ReturnMode::Matches, None);
    let root = PathBuf::from(temp_path);
    file_search::execute(&options_ignore, &root, &mut ctx_ignore);

    let results_ignore = ctx_ignore.results().blocking_read();
    println!("Test: aggressive gitignore, no_ignore=false");
    println!("  Found {} files:", results_ignore.len());
    for r in results_ignore.iter() {
        println!("    - {}", r.file);
    }

    // With .gitignore respected, should find 0 .md files (all ignored by *)
    assert_eq!(
        results_ignore.len(),
        0,
        "Should find 0 files when aggressive .gitignore is respected"
    );

    // === Test with no_ignore=true ===
    let options_no_ignore = make_options(&temp_path.to_string_lossy(), ".*\\.md$", true);
    let mut ctx_no_ignore = SearchContext::new(100, ReturnMode::Matches, None);
    file_search::execute(&options_no_ignore, &root, &mut ctx_no_ignore);

    let results_no_ignore = ctx_no_ignore.results().blocking_read();
    println!("Test: aggressive gitignore, no_ignore=true");
    println!("  Found {} files:", results_no_ignore.len());
    for r in results_no_ignore.iter() {
        println!("    - {}", r.file);
    }

    // With no_ignore=true, should find ALL 3 .md files
    assert_eq!(
        results_no_ignore.len(),
        3,
        "Should find 3 files when .gitignore is bypassed, found: {:?}",
        results_no_ignore.iter().map(|r| &r.file).collect::<Vec<_>>()
    );
}

/// Test: Direct WalkBuilder test to verify ignore crate behavior
/// This bypasses the kodegen wrapper to confirm the ignore crate works correctly
#[test]
fn test_walkbuilder_directly_with_no_ignore() {
    use ignore::WalkBuilder;

    // Create temp directory with .gitignore
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");
    fs::write(temp_path.join("test.md"), "# Test").expect("Failed to write test.md");
    fs::write(temp_path.join(".gitignore"), "*\n!.gitignore\n").expect("Failed to write .gitignore");

    println!("\n=== Test: WalkBuilder directly ===");

    // Test 1: With gitignore respected (default)
    let walker_with_ignore = WalkBuilder::new(temp_path);
    // Default is git_ignore(true), so .gitignore is respected

    let mut found_with_ignore = Vec::new();
    for e in walker_with_ignore.build().flatten() {
        if e.path().is_file() {
            found_with_ignore.push(e.path().to_path_buf());
        }
    }

    println!("With gitignore respected (default):");
    for f in &found_with_ignore {
        println!("  - {}", f.display());
    }

    // Should NOT find test.md (ignored by *)
    let found_md_with_ignore = found_with_ignore.iter().any(|f| f.extension().is_some_and(|e| e == "md"));
    assert!(
        !found_md_with_ignore,
        "With gitignore respected, should NOT find .md files. Found: {:?}",
        found_with_ignore
    );

    // Test 2: With gitignore bypassed
    let mut walker_no_ignore = WalkBuilder::new(temp_path);
    walker_no_ignore
        .git_ignore(false)  // Disable .gitignore
        .ignore(false)      // Disable .ignore
        .parents(false)     // Don't read parent ignore files
        .git_global(false)  // Disable global gitignore
        .git_exclude(false); // Disable .git/info/exclude

    let mut found_no_ignore = Vec::new();
    for e in walker_no_ignore.build().flatten() {
        if e.path().is_file() {
            found_no_ignore.push(e.path().to_path_buf());
        }
    }

    println!("With gitignore bypassed:");
    for f in &found_no_ignore {
        println!("  - {}", f.display());
    }

    // Should find test.md
    let found_md_no_ignore = found_no_ignore.iter().any(|f| f.extension().is_some_and(|e| e == "md"));
    assert!(
        found_md_no_ignore,
        "With gitignore bypassed, SHOULD find .md files. Found: {:?}",
        found_no_ignore
    );
}
