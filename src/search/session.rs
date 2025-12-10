//! Search session with timeout and background continuation
//!
//! Manages a single search instance with background task spawning,
//! timeout handling, and state persistence across MCP requests.

use anyhow::Result;
use kodegen_mcp_schema::filesystem::{FsSearchArgs, FsSearchOutput, FsSearchResult, FsSearchResultType};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::manager::context::SearchContext;
use super::manager::{content_search, file_search};
use super::types::{SearchSessionOptions, SearchIn, CaseMode, BoundaryMode, SearchResult, SearchResultType};

/// Internal snapshot data for registry access
#[derive(Debug, Clone)]
pub struct SearchStateSnapshot {
    pub pattern: String,
    pub path: String,
    pub match_count: usize,
    pub files_searched: usize,
    pub completed: bool,
    pub duration_ms: u64,
}

/// Search state snapshot
#[derive(Debug, Clone)]
struct SearchState {
    // Metadata
    pattern: String,
    path: String,
    search_in: SearchIn,

    // Progress
    results: Vec<SearchResult>,
    match_count: usize,
    files_searched: usize,
    error_count: usize,
    errors: Vec<String>,

    // Status
    completed: bool,
    success: bool,
    exit_code: Option<i32>,
    error: Option<String>,

    // Timing
    start_time: std::time::Instant,
}

impl SearchState {
    fn new(pattern: String, path: String, search_in: SearchIn) -> Self {
        Self {
            pattern,
            path,
            search_in,
            results: Vec::new(),
            match_count: 0,
            files_searched: 0,
            error_count: 0,
            errors: Vec::new(),
            completed: false,
            success: false,
            exit_code: None,
            error: None,
            start_time: std::time::Instant::now(),
        }
    }
}

/// Convert internal SearchResultType to schema FsSearchResultType
fn convert_result_type(rt: &SearchResultType) -> FsSearchResultType {
    match rt {
        SearchResultType::File => FsSearchResultType::File,
        SearchResultType::Content => FsSearchResultType::Content,
        SearchResultType::FileList => FsSearchResultType::FileList,
    }
}

/// Convert internal SearchResult to schema FsSearchResult
fn convert_search_result(result: &SearchResult) -> FsSearchResult {
    FsSearchResult {
        file: result.file.clone(),
        line: result.line,
        r#match: result.r#match.clone(),
        r#type: convert_result_type(&result.r#type),
        is_context: result.is_context,
        is_binary: result.is_binary,
        binary_suppressed: result.binary_suppressed,
    }
}

/// Search session - manages background search with timeout
pub struct SearchSession {
    search_id: u32,
    state: Arc<Mutex<SearchState>>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl SearchSession {
    /// Create a new search session
    pub fn new(search_id: u32) -> Self {
        Self {
            search_id,
            state: Arc::new(Mutex::new(SearchState::new(
                String::new(),
                String::new(),
                SearchIn::Content,
            ))),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a snapshot of the current state for the registry
    pub async fn get_snapshot(&self) -> SearchStateSnapshot {
        let state = self.state.lock().await;
        SearchStateSnapshot {
            pattern: state.pattern.clone(),
            path: state.path.clone(),
            match_count: state.match_count,
            files_searched: state.files_searched,
            completed: state.completed,
            duration_ms: state.start_time.elapsed().as_millis() as u64,
        }
    }

    /// Execute search with timeout support
    pub async fn execute_search_with_timeout(
        &self,
        args: FsSearchArgs,
        await_completion_ms: u64,
        client_pwd: Option<PathBuf>,
    ) -> Result<FsSearchOutput> {
        let start = std::time::Instant::now();

        // Extract required fields
        let path = args
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("path required for SEARCH action"))?
            .clone();
        let pattern = args
            .pattern
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("pattern required for SEARCH action"))?
            .clone();

        // Update initial state
        {
            let mut state = self.state.lock().await;
            state.pattern = pattern.clone();
            state.path = path.clone();
            state.search_in = args.search_in;
            state.start_time = start;
            state.completed = false;
        }

        // Build search options (reuse existing logic)
        let case_mode = args
            .ignore_case
            .map(|ic| {
                if ic {
                    CaseMode::Insensitive
                } else {
                    CaseMode::Sensitive
                }
            })
            .unwrap_or(args.case_mode);

        let boundary_mode = if args.word_boundary == Some(true) {
            Some(BoundaryMode::Word)
        } else {
            args.boundary_mode.as_deref().and_then(|s| match s {
                "word" => Some(BoundaryMode::Word),
                "line" => Some(BoundaryMode::Line),
                _ => None,
            })
        };

        let options = SearchSessionOptions {
            root_path: path.clone(),
            pattern: pattern.clone(),
            search_in: args.search_in,
            file_pattern: args.file_pattern,
            r#type: args.r#type,
            type_not: args.type_not,
            case_mode,
            max_results: args.max_results,
            include_hidden: args.include_hidden,
            no_ignore: args.no_ignore,
            context: args.context,
            before_context: args.before_context,
            after_context: args.after_context,
            timeout_ms: args.timeout_ms,
            early_termination: args.early_termination,
            literal_search: args.literal_search,
            boundary_mode,
            return_only: args.return_only,
            invert_match: args.invert_match,
            engine: args.engine,
            preprocessor: args.preprocessor,
            preprocessor_globs: args.preprocessor_globs,
            search_zip: args.search_zip,
            binary_mode: args.binary_mode,
            multiline: args.multiline,
            max_filesize: args.max_filesize,
            max_depth: args.max_depth,
            only_matching: args.only_matching,
            sort_by: args.sort_by,
            sort_direction: args.sort_direction,
            encoding: args.encoding,
        };

        // Spawn background search task
        let state_clone = self.state.clone();
        let search_task = tokio::spawn(async move {
            let max_results = options.max_results.map(|v| v as usize).unwrap_or(usize::MAX);
            let return_only = options.return_only;
            let root = PathBuf::from(&options.root_path);
            let search_in = options.search_in;

            // Execute search in blocking threadpool
            let result = tokio::task::spawn_blocking(move || {
                let mut ctx = SearchContext::new(max_results, return_only, client_pwd);

                match search_in {
                    SearchIn::Content => content_search::execute(&options, &root, &mut ctx),
                    SearchIn::Filenames => file_search::execute(&options, &root, &mut ctx),
                }

                // Extract all data from SearchContext
                let results = ctx.results().blocking_read().clone();
                let errors = ctx.errors().blocking_read().clone();
                let total_matches = ctx.total_matches();
                let total_files = ctx.total_files();
                let error_count = ctx.error_count_value();
                let is_complete = ctx.is_complete;
                let is_error = ctx.is_error;
                let error = ctx.error.clone();

                (
                    results,
                    errors,
                    total_matches,
                    total_files,
                    error_count,
                    is_complete,
                    is_error,
                    error,
                )
            })
            .await;

            // Update state with results
            match result {
                Ok((
                    results,
                    errors,
                    total_matches,
                    total_files,
                    error_count,
                    is_complete,
                    is_error,
                    error,
                )) => {
                    let mut state = state_clone.lock().await;

                    // Store results directly
                    state.results = results;
                    state.match_count = total_matches;
                    state.files_searched = total_files;
                    state.error_count = error_count;
                    state.errors = errors.iter().map(|e| format!("{:?}", e)).collect();
                    state.completed = is_complete;
                    state.success = !is_error;
                    state.exit_code = Some(if is_error { 1 } else { 0 });
                    state.error = error;
                }
                Err(e) => {
                    let mut state = state_clone.lock().await;
                    state.completed = true;
                    state.success = false;
                    state.exit_code = Some(1);
                    state.error = Some(format!("Search task panicked: {}", e));
                }
            }
        });

        // Fire-and-forget mode (await_completion_ms == 0)
        if await_completion_ms == 0 {
            *self.handle.lock().await = Some(search_task);

            return Ok(FsSearchOutput {
                search: Some(self.search_id),
                output: "\x1b[36m[Search] Background search started\x1b[0m\n• Use action=READ to check progress".to_string(),
                pattern,
                path,
                results: Vec::new(),
                searches: Vec::new(),
                match_count: 0,
                files_searched: 0,
                error_count: 0,
                errors: Vec::new(),
                duration_ms: start.elapsed().as_millis() as u64,
                completed: false,
                success: true,
                exit_code: None,
                error: None,
            });
        }

        // Wait with timeout
        let timeout_duration = std::time::Duration::from_millis(await_completion_ms);
        let timeout_result = tokio::time::timeout(timeout_duration, search_task).await;

        // Read final state
        let state = self.state.lock().await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Convert results to schema type
        let results: Vec<FsSearchResult> = state.results.iter().map(convert_search_result).collect();

        match timeout_result {
            Err(_) => {
                // Timeout - return partial results
                Ok(FsSearchOutput {
                    search: Some(self.search_id),
                    output: format!(
                        "\x1b[33m[Progress] Found {} matches in {} files so far\x1b[0m\n\n\x1b[36m[Search] Still running\x1b[0m\n• Use action=READ for more results",
                        state.match_count, state.files_searched
                    ),
                    pattern: state.pattern.clone(),
                    path: state.path.clone(),
                    results,
                    searches: Vec::new(),
                    match_count: state.match_count,
                    files_searched: state.files_searched,
                    error_count: state.error_count,
                    errors: Vec::new(),
                    duration_ms,
                    completed: false,
                    success: true,
                    exit_code: None,
                    error: None,
                })
            }
            Ok(Ok(_)) => {
                // Completed successfully
                Ok(FsSearchOutput {
                    search: Some(self.search_id),
                    output: format!(
                        "\x1b[32m[Complete] Search completed\x1b[0m\n• {} matches in {} files",
                        state.match_count, state.files_searched
                    ),
                    pattern: state.pattern.clone(),
                    path: state.path.clone(),
                    results,
                    searches: Vec::new(),
                    match_count: state.match_count,
                    files_searched: state.files_searched,
                    error_count: state.error_count,
                    errors: state.errors.clone(),
                    duration_ms,
                    completed: true,
                    success: state.success,
                    exit_code: state.exit_code,
                    error: state.error.clone(),
                })
            }
            Ok(Err(e)) => {
                // Task panicked
                Ok(FsSearchOutput {
                    search: Some(self.search_id),
                    output: format!("\x1b[31m[Error] Search failed\x1b[0m\n• {}", e),
                    pattern: state.pattern.clone(),
                    path: state.path.clone(),
                    results: Vec::new(),
                    searches: Vec::new(),
                    match_count: 0,
                    files_searched: 0,
                    error_count: 1,
                    errors: Vec::new(),
                    duration_ms,
                    completed: true,
                    success: false,
                    exit_code: Some(1),
                    error: Some(format!("{}", e)),
                })
            }
        }
    }

    /// Read current search state without re-executing
    pub async fn read_current_state(&self) -> Result<FsSearchOutput> {
        let state = self.state.lock().await;
        let duration_ms = state.start_time.elapsed().as_millis() as u64;

        // Convert results to schema type
        let results: Vec<FsSearchResult> = state.results.iter().map(convert_search_result).collect();

        Ok(FsSearchOutput {
            search: Some(self.search_id),
            output: if state.completed {
                format!(
                    "\x1b[32m[Complete] Search completed\x1b[0m\n• {} matches in {} files",
                    state.match_count, state.files_searched
                )
            } else {
                format!(
                    "\x1b[33m[Progress] Search in progress\x1b[0m\n• {} matches in {} files so far",
                    state.match_count, state.files_searched
                )
            },
            pattern: state.pattern.clone(),
            path: state.path.clone(),
            results,
            searches: Vec::new(),
            match_count: state.match_count,
            files_searched: state.files_searched,
            error_count: state.error_count,
            errors: state.errors.clone(),
            duration_ms,
            completed: state.completed,
            success: state.success,
            exit_code: state.exit_code,
            error: state.error.clone(),
        })
    }

    /// Cancel the search
    pub async fn cancel(&self) -> Result<()> {
        if let Some(handle) = self.handle.lock().await.as_ref() {
            handle.abort();
        }

        let mut state = self.state.lock().await;
        state.completed = true;
        state.success = false;
        state.exit_code = Some(130); // SIGINT

        Ok(())
    }
}
