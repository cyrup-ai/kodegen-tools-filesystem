use kodegen_mcp_schema::filesystem::{FsStartSearchArgs, FsStartSearchPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::sync::Arc;

use super::{SearchManager, SearchSessionOptions, SearchIn, ReturnMode, CaseMode, BoundaryMode};

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct StartSearchTool {
    manager: Arc<SearchManager>,
}

impl StartSearchTool {
    #[must_use]
    pub fn new(manager: Arc<SearchManager>) -> Self {
        Self { manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for StartSearchTool {
    type Args = FsStartSearchArgs;
    type PromptArgs = FsStartSearchPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_START_SEARCH
    }

    fn description() -> &'static str {
        "🚀 BLAZING-FAST SEARCH (10-100x faster than grep). Respects .gitignore automatically.\n\n\
         QUICK START - Smart defaults mean most searches just work:\n\
         • Find TODO comments:              fs_start_search(pattern='TODO')\n\
         • Find package.json:               fs_start_search(pattern='package.json', search_in='filenames')\n\
         • Get paths with 'error':          fs_start_search(pattern='error', return_only='paths')\n\
         • Count imports per file:          fs_start_search(pattern='^import', return_only='counts')\n\n\
         PARAMETERS - TWO INDEPENDENT CONTROLS:\n\n\
         1. search_in: WHERE to search (default: 'content')\n\
            • 'content' - Search inside file contents (default, like `rg PATTERN`)\n\
            • 'filenames' - Search file names/paths\n\n\
         2. return_only: WHAT to return (default: 'matches')\n\
            • 'matches' - Full details: path, line, content (default, like `rg PATTERN`)\n\
            • 'paths' - Just unique file paths (like `rg -l PATTERN`)\n\
            • 'counts' - Match counts per file (like `rg -c PATTERN`)\n\n\
         These are INDEPENDENT - you can combine any search_in with any return_only:\n\
         • search_in='content', return_only='matches' → matching lines with context\n\
         • search_in='content', return_only='paths' → files containing matches\n\
         • search_in='content', return_only='counts' → match counts per file\n\
         • search_in='filenames', return_only='matches' → matching files with metadata\n\n\
         ════════════════════════════════════════════════════════════════════════\n\n\
         SEARCH STRATEGY GUIDE:\n\
         Choose the right search location based on what the user is looking for:\n\n\
         USE search_in=\"filenames\" WHEN:\n\
         - User asks for specific files: \"find package.json\", \"locate config files\"\n\
         - Pattern looks like a filename: \"*.js\", \"README.md\", \"test-*.tsx\"\n\
         - User wants to find files by name/extension: \"all TypeScript files\", \"Python scripts\"\n\
         - Looking for configuration/setup files: \".env\", \"dockerfile\", \"tsconfig.json\"\n\n\
         USE search_in=\"content\" (DEFAULT) WHEN:\n\
         - User asks about code/logic: \"authentication logic\", \"error handling\", \"API calls\"\n\
         - Looking for functions/variables: \"getUserData function\", \"useState hook\"\n\
         - Searching for text/comments: \"TODO items\", \"FIXME comments\", \"documentation\"\n\
         - Finding patterns in code: \"console.log statements\", \"import statements\"\n\
         - User describes functionality: \"components that handle login\", \"files with database queries\"\n\n\
         WHEN UNSURE OR USER REQUEST IS AMBIGUOUS:\n\
         Run TWO searches in parallel - one for filenames and one for content:\n\n\
         Example approach for ambiguous queries like \"find authentication stuff\":\n\
         1. Start filename search: search_in=\"filenames\", pattern=\"auth\"\n\
         2. Simultaneously start content search: search_in=\"content\", pattern=\"authentication\"\n\
         3. Present combined results: \"Found 3 auth-related files and 8 files containing authentication code\"\n\n\
         SEARCH LOCATIONS:\n\
         - search_in=\"filenames\": Find files by name (pattern matches file names)\n\
         - search_in=\"content\": Search inside files for text patterns (default)\n\n\
         PATTERN MATCHING MODES:\n\
         - Default (literal_search=false): Patterns are regex (matches ripgrep behavior)\n\
         - Literal mode (literal_search=true): Patterns are treated as exact strings\n\
         - Smart case (case_mode=\"smart\"): Auto case-insensitive for all-lowercase patterns\n\
         - Boundary modes (boundary_mode parameter):\n\
           * null/omitted: Match pattern anywhere (substring matching, default)\n\
           * \"word\": Match whole words only (uses \\b anchors)\n\
             - Content: 'test' matches 'test()' but not 'testing'\n\
             - Files: 'lib' matches 'lib.rs' but not 'libtest.rs'\n\
           * \"line\": Match complete lines only (uses ^ and $ anchors)\n\
             - Content: 'error' matches 'error' alone but not 'this error happened'\n\
             - Files: Less useful but supported\n\
         Note: Simple strings like \"start_crawl\" work as regex and will match literally\n\n\
         IMPORTANT PARAMETERS:\n\
         - search_in: Where to search (\"content\" or \"filenames\", default: \"content\")\n\
         - return_only: What to return (\"matches\", \"paths\", or \"counts\", default: \"matches\")\n\
         - pattern: What to search for (file names OR content text)\n\
         - literal_search: Use exact string matching instead of regex (default: false)\n\
         - boundary_mode: \"word\", \"line\", or null for pattern boundaries (default: null)\n\
         - multiline (default: false): Enable multiline pattern matching (rg --multiline)\n\
           * Allows patterns to span multiple lines\n\
           * Makes '.' match newline characters\n\
           * Essential for structural code analysis\n\
         - case_mode: \"sensitive\", \"insensitive\", or \"smart\" (default: \"sensitive\")\n\
           Smart case: case-insensitive if pattern is all lowercase, sensitive otherwise\n\
         - file_pattern: Optional filter to limit search to specific file types (e.g., \"*.js\", \"package.json\")\n\
         - early_termination: Stop search early when exact filename match is found (optional: defaults to true for filename searches, false for content searches)\n\
         - only_matching: Return only the matched portion of text, not entire lines (rg -o)\n\
           Only works with search_in=\"content\". Perfect for data extraction.\n\
           Examples: Extract URLs, function names, version numbers, email addresses\n\
         - max_depth: Limit directory traversal depth (default: unlimited)\n\
           * Essential for performance in monorepos with deep dependency trees (node_modules, vendor, target)\n\
           * Example: max_depth=3 searches root + 3 levels, skipping deeper directories\n\
           * Common values: 1 (root+children only), 3-4 (avoid deep node_modules/dependencies)\n\
           * Matches ripgrep's --max-depth flag\n\
           * Can provide 10-25x speedup by avoiding irrelevant deep directories\n\n\
         - max_filesize: Skip files larger than specified size in bytes (default: None/unlimited)\n\
           * Matches ripgrep's --max-filesize flag\n\
           * Essential for performance: avoids huge minified bundles, lock files, generated code\n\
           * Recommended: 1048576 (1MB) for most searches\n\
           * Skips: bundle.min.js (15MB), package-lock.json (12MB), Cargo.lock (1-10MB)\n\
           * Common values:\n\
             - 102400 (100KB): Ultra-fast, only small source files\n\
             - 1048576 (1MB): Recommended - Skip minified bundles and lock files\n\
             - 5242880 (5MB): Conservative - Allow large source, skip huge bundles\n\
           * Can provide 10-30x speedup by avoiding huge files that waste search time\n\
           * Use with max_depth for maximum performance in large projects\n\
         - encoding: Text encoding for file content (default: \"auto\")\n\
           * Supports any encoding_rs name: utf8, utf16le, utf16be, latin1, shiftjis, gb2312, euckr, etc.\n\
           * Use when: Mojibake in results, legacy codebases, international projects\n\
           * Examples: encoding=\"utf16le\" for Windows files, encoding=\"shiftjis\" for Japanese code\n\n\
         DECISION EXAMPLES:\n\
         - \"find package.json\" → search_in=\"filenames\", pattern=\"package.json\" (specific file)\n\
         - \"find authentication components\" → search_in=\"content\", pattern=\"authentication\" (looking for functionality)\n\
         - \"locate all React components\" → search_in=\"filenames\", pattern=\"*.tsx\" or \"*.jsx\" (file pattern)\n\
         - \"find TODO comments\" → pattern=\"TODO\" (defaults to content search)\n\
         - \"show me login files\" → AMBIGUOUS → run both: filenames with \"login\" AND content with \"login\"\n\
         - \"find config\" → AMBIGUOUS → run both: config files AND files containing config code\n\n\
         COMPREHENSIVE SEARCH EXAMPLES:\n\
         - Find package.json files: search_in=\"filenames\", pattern=\"package.json\"\n\
         - Find all JS files: search_in=\"filenames\", pattern=\"*.js\"\n\
         - Search for TODO in code: pattern=\"TODO\", file_pattern=\"*.js|*.ts\" (content search is default)\n\
         - Search for exact code: pattern=\"toast.error('test')\", literal_search=true\n\
         - Search whole words: pattern=\"test\", boundary_mode=\"word\"\n\
           (matches 'test()' and 'test ' but not 'testing' or 'attest')\n\
         - Find exact filename: search_in=\"filenames\", pattern=\"lib\", boundary_mode=\"word\"\n\
           (matches 'lib.rs' but not 'libtest.rs')\n\
         - Match complete lines: pattern=\"error\", boundary_mode=\"line\"\n\
           (matches 'error' alone but not 'this error happened' or '  error  ')\n\
         - Ambiguous request \"find auth stuff\": Run two searches:\n\
           1. search_in=\"filenames\", pattern=\"auth\"\n\
           2. search_in=\"content\", pattern=\"authentication\"\n\
         - Extract URLs: pattern=\"https?://[^\\\\s]+\", only_matching=true\n\
           (returns just \"https://example.com\" not full line)\n\
         - Extract function names: pattern=\"fn (\\\\w+)\\\\(\", only_matching=true\n\
         - Extract version numbers: pattern=\"\\\\d+\\\\.\\\\d+\\\\.\\\\d+\", only_matching=true\n\n\
         PRO TIP: When user requests are ambiguous about whether they want files or content,\n\
         run both searches concurrently and combine results for comprehensive coverage.\n\n\
         Unlike regular search tools, this starts a background search process and returns\n\
         immediately with a session ID. Use get_search_results to get results as they\n\
         come in, and stop_search to stop the search early if needed.\n\n\
         Perfect for large directories where you want to see results immediately and\n\
         have the option to cancel if the search takes too long or you find what you need.\n\n\
         IMPORTANT: Always use absolute paths for reliability. Paths are automatically normalized regardless of slash direction. Relative paths may fail as they depend on the current working directory. Tilde paths (~/...) might not work in all contexts. Unless the user explicitly asks for relative paths, use absolute paths."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        // Handle backward compatibility: ignore_case overrides case_mode if present
        let case_mode = if let Some(ignore_case) = args.ignore_case {
            if ignore_case {
                CaseMode::Insensitive
            } else {
                CaseMode::Sensitive
            }
        } else {
            args.case_mode
        };

        // Handle backward compatibility: word_boundary overrides boundary_mode if present
        let boundary_mode = if let Some(true) = args.word_boundary {
            log::warn!("word_boundary is deprecated, use boundary_mode='word' instead");
            Some(BoundaryMode::Word)
        } else {
            // Parse boundary_mode string to enum
            match args.boundary_mode.as_deref() {
                None => None,
                Some("word") => Some(BoundaryMode::Word),
                Some("line") => Some(BoundaryMode::Line),
                Some(other) => {
                    return Err(McpError::InvalidArguments(format!(
                        "Invalid boundary_mode '{other}'. Must be 'word', 'line', or null"
                    )));
                }
            }
        };

        // Validate only_matching only works with content search
        if args.only_matching && args.search_in != SearchIn::Content {
            return Err(McpError::InvalidArguments(
                "only_matching can only be used with search_in 'content'".to_string(),
            ));
        }

        // Warn if only_matching + invert_match (illogical combination)
        if args.only_matching && args.invert_match {
            log::warn!(
                "only_matching + invert_match: nothing to extract from non-matches, ignoring only_matching"
            );
        }

        // Warn if only_matching with non-Matches return mode (only_matching has no effect)
        if args.only_matching && args.return_only != ReturnMode::Matches {
            log::warn!(
                "only_matching with return_only={:?}: non-Matches modes don't have match text, ignoring only_matching",
                args.return_only
            );
        }

        let options = SearchSessionOptions {
            root_path: args.path.clone(),
            pattern: args.pattern.clone(),
            search_in: args.search_in,
            file_pattern: args.file_pattern,
            r#type: args.r#type,
            type_not: args.type_not,
            case_mode, // Changed from ignore_case
            max_results: args.max_results,
            include_hidden: args.include_hidden,
            no_ignore: args.no_ignore,
            context: args.context,
            before_context: args.before_context,
            after_context: args.after_context,
            timeout_ms: args.timeout_ms,
            early_termination: args.early_termination,
            literal_search: args.literal_search,
            boundary_mode, // Changed from word_boundary
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

        let response = self.manager.start_search(options).await?;

        let mut contents = Vec::new();

        // Content 1: Human-readable summary
        let status = if response.is_complete {
            if response.is_error {
                "failed"
            } else {
                "completed"
            }
        } else {
            "running"
        };

        let search_type_str = match args.search_in {
            SearchIn::Filenames => "filenames",
            SearchIn::Content => "content",
        };

        let summary = format!(
            "\x1b[36m󰺮 Search started: {}\x1b[0m\n\
             󰓎 Search ID: {} · Pattern: \"{}\"\n\
             󰘖 Status: {} · {} initial results from {}",
            search_type_str,
            response.search_id,
            args.pattern,
            status,
            response.total_results,
            args.path
        );
        contents.push(Content::text(summary));

        // Content 2: JSON metadata
        let metadata = json!({
            "success": true,
            "search_id": response.search_id,
            "is_complete": response.is_complete,
            "is_error": response.is_error,
            "results": response.results,
            "total_results": response.total_results,
            "runtime_ms": response.runtime_ms,
            "error_count": response.error_count,
            "max_results": response.max_results,
            "results_limited": response.results_limited,
            "pattern": args.pattern,
            "path": args.path,
            "search_type": search_type_str,
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I use streaming search?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The start_search tool starts a background search that returns results progressively:\n\n\
                     1. File search:\n\
                        start_search({\n\
                          \"path\": \"/path/to/search\",\n\
                          \"pattern\": \"package.json\",\n\
                          \"search_type\": \"files\"\n\
                        })\n\n\
                     2. Content search:\n\
                        start_search({\n\
                          \"path\": \".\",\n\
                          \"pattern\": \"TODO\",\n\
                          \"search_type\": \"content\",\n\
                          \"file_pattern\": \"*.rs\"\n\
                        })\n\n\
                     Returns search_id immediately. Use get_search_results to fetch results.",
                ),
            },
        ])
    }
}
