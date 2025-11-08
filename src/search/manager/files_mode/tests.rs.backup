//!
//! Tests for files mode

#[cfg(test)]
mod tests {
    use super::super::execute::execute;
    use super::super::super::context::SearchContext;
    use super::super::super::super::types::{CaseMode, SearchType, SearchSessionOptions, SearchResultType, SearchOutputMode};

    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
    use std::time::Instant;
    use tempfile::TempDir;
    use tokio::sync::{RwLock, watch};

    #[test]
    fn test_files_mode_basic() {
        // Create temp directory with some files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "content1").expect("Failed to write file1");
        fs::write(temp_path.join("file2.rs"), "content2").expect("Failed to write file2");

        // Create subdirectory with file
        fs::create_dir(temp_path.join("subdir")).expect("Failed to create subdir");
        fs::write(temp_path.join("subdir/file3.md"), "content3").expect("Failed to write file3");

        // Create options for files mode
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(), // Pattern ignored in files mode
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::super::types::Engine::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute files mode
        execute(&options, temp_path, &mut ctx);

        // Verify results
        let results = ctx.results.blocking_read();
        assert_eq!(results.len(), 3, "Should find 3 files");

        // Verify all results are FileList type
        for result in results.iter() {
            assert!(matches!(result.r#type, SearchResultType::FileList));
            assert!(result.line.is_none());
            assert!(result.r#match.is_none());
        }

        // Verify completion
        assert!(ctx.is_complete.load(Ordering::Acquire));
    }

    #[test]
    fn test_files_mode_respects_gitignore() {
        // Create temp directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create .git directory to make it a git repository (required for gitignore)
        fs::create_dir(temp_path.join(".git")).expect("Failed to create .git dir");

        // Create test files
        fs::write(temp_path.join("tracked.rs"), "content").expect("Failed to write tracked file");
        fs::write(temp_path.join("ignored.log"), "content").expect("Failed to write ignored file");

        // Create .gitignore
        fs::write(temp_path.join(".gitignore"), "*.log\n").expect("Failed to write .gitignore");

        // Create options
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::super::types::Engine::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - should only find tracked.rs, not ignored.log
        let results = ctx.results.blocking_read();
        assert_eq!(
            results.len(),
            1,
            "Should find only 1 file (ignored.log should be excluded)"
        );
        assert!(results[0].file.contains("tracked.rs"));
    }

    #[test]
    fn test_files_mode_type_filter() {
        // Create temp directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create files of different types
        fs::write(temp_path.join("code.rs"), "fn main() {}").expect("Failed to write rs file");
        fs::write(temp_path.join("doc.md"), "# Title").expect("Failed to write md file");
        fs::write(temp_path.join("data.json"), "{}").expect("Failed to write json file");

        // Create options with type filter for rust files only
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec!["rust".to_string()],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::super::types::Engine::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: None,
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - should only find .rs file
        let results = ctx.results.blocking_read();
        assert_eq!(results.len(), 1, "Should find only 1 rust file");
        assert!(results[0].file.contains("code.rs"));
    }

    #[test]
    fn test_files_mode_max_depth() {
        // Create temp directory with nested structure
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create files at different depths
        fs::write(temp_path.join("root.txt"), "root").expect("Failed to write root file");

        fs::create_dir(temp_path.join("level1")).expect("Failed to create level1");
        fs::write(temp_path.join("level1/file1.txt"), "l1").expect("Failed to write level1 file");

        fs::create_dir(temp_path.join("level1/level2")).expect("Failed to create level2");
        fs::write(temp_path.join("level1/level2/file2.txt"), "l2")
            .expect("Failed to write level2 file");

        fs::create_dir(temp_path.join("level1/level2/level3")).expect("Failed to create level3");
        fs::write(temp_path.join("level1/level2/level3/file3.txt"), "l3")
            .expect("Failed to write level3 file");

        // Create options with max_depth = 2
        let options = SearchSessionOptions {
            root_path: temp_path.to_string_lossy().to_string(),
            pattern: String::new(),
            search_type: SearchType::Files,
            file_pattern: None,
            r#type: vec![],
            type_not: vec![],
            case_mode: CaseMode::Sensitive,
            max_results: Some(100),
            include_hidden: false,
            no_ignore: false,
            context: 0,
            before_context: None,
            after_context: None,
            timeout_ms: None,
            early_termination: None,
            literal_search: false,
            boundary_mode: None,
            output_mode: SearchOutputMode::Full,
            invert_match: false,
            engine: super::super::super::super::types::Engine::Auto,
            preprocessor: None,
            preprocessor_globs: vec![],
            search_zip: false,
            binary_mode: super::super::super::super::types::BinaryMode::Auto,
            multiline: false,
            max_filesize: None,
            max_depth: Some(2), // Limit to 2 levels deep
            only_matching: false,
            list_files_only: true,
            sort_by: None,
            sort_direction: None,
            encoding: None,
        };

        // Create context
        let (_tx, rx) = watch::channel(false);
        let (first_result_tx, _first_result_rx) = watch::channel(false);
        let start_time = Instant::now();
        let mut ctx = SearchContext {
            results: Arc::new(RwLock::new(Vec::new())),
            is_complete: Arc::new(AtomicBool::new(false)),
            total_matches: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
            last_read_time_atomic: Arc::new(AtomicU64::new(0)),
            cancellation_rx: rx,
            first_result_tx,
            was_incomplete: Arc::new(RwLock::new(false)),
            error_count: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(RwLock::new(Vec::new())),
            is_error: Arc::new(RwLock::new(false)),
            error: Arc::new(RwLock::new(None)),
            output_mode: SearchOutputMode::Full,
            seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
            file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            start_time,
        };

        // Execute
        execute(&options, temp_path, &mut ctx);

        // Verify results - max_depth=2 in ignore crate means:
        // depth 0 (root) + depth 1 (level1) = 2 levels
        // So we get: root.txt + level1/file1.txt = 2 files
        // level1/level2 (depth 2) is excluded
        let results = ctx.results.blocking_read();
        assert!(
            results.len() >= 2,
            "Should find at least 2 files (root and level1)"
        );
        assert!(
            results.len() <= 3,
            "Should not find more than 3 files (level3 should be excluded)"
        );

        // Verify file3.txt at level3 is NOT included
        let has_level3 = results.iter().any(|r| r.file.contains("level3"));
        assert!(!has_level3, "Should not include files beyond max_depth");
    }
}
