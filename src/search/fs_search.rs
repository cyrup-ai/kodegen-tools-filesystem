use kodegen_mcp_schema::filesystem::{FsSearchAction, FsSearchArgs, FsSearchOutput, SearchPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use std::sync::Arc;

use super::registry::SearchRegistry;

// ============================================================================
// TOOL STRUCT
// ============================================================================

/// Filesystem search tool with elite terminal pattern
#[derive(Clone)]
pub struct FsSearchTool {
    registry: Arc<SearchRegistry>,
}

impl FsSearchTool {
    #[must_use]
    pub fn new(registry: Arc<SearchRegistry>) -> Self {
        Self { registry }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for FsSearchTool {
    type Args = FsSearchArgs;
    type Prompts = SearchPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_SEARCH
    }

    fn description() -> &'static str {
        "🚀 BLAZING-FAST SEARCH (10-100x faster than grep). Respects .gitignore automatically. Built on ripgrep.\n\n\
         QUICK START:\n\
         • Find files: fs_search(pattern='package.json', search_in='filenames')\n\
         • Find TODO comments: fs_search(pattern='TODO')\n\
         • Get paths with 'error': fs_search(pattern='error', return_only='paths')\n\
         • Count imports per file: fs_search(pattern='^import', return_only='counts')\n\n\
         COMPREHENSIVE PARAMETERS:\n\n\
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
             - Files: Less useful but supported\n\n\
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
         COMPREHENSIVE SEARCH EXAMPLES:\n\
         - Find package.json files: search_in=\"filenames\", pattern=\"package.json\"\n\
         - Find all JS files: search_in=\"filenames\", pattern=\"*.js\"\n\
         - Search for TODO in code: pattern=\"TODO\", file_pattern=\"*.js|*.ts\" (content search is default)\n\
         - Search for exact code: pattern=\"toast.error('test')\", literal_search=true\n\
         - Search whole words: pattern=\"test\", boundary_mode=\"word\"\n\
         - Find exact filename: search_in=\"filenames\", pattern=\"lib\", boundary_mode=\"word\"\n\
         - Match complete lines: pattern=\"error\", boundary_mode=\"line\"\n\
         - Extract URLs: pattern=\"https?://[^\\\\s]+\", only_matching=true\n\
         - Extract function names: pattern=\"fn (\\\\w+)\\\\(\", only_matching=true\n\
         - Extract version numbers: pattern=\"\\\\d+\\\\.\\\\d+\\\\.\\\\d+\", only_matching=true\n\n\
         PRO TIP: When user requests are ambiguous about whether they want files or content,\n\
         run both searches concurrently and combine results for comprehensive coverage.\n\n\
         This tool performs a blocking search and returns ALL results immediately.\n\
         Perfect for most search tasks where you want complete results in a single response.\n\n\
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

    async fn execute(
        &self,
        args: Self::Args,
        ctx: ToolExecutionContext,
    ) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let connection_id = ctx.connection_id().unwrap_or("default");

        // Extract client's pwd from ToolExecutionContext
        let client_pwd = ctx.pwd().map(|p| p.to_path_buf());

        // Dispatch based on action - all paths return typed FsSearchOutput
        let output: FsSearchOutput = match args.action {
            FsSearchAction::List => {
                // List all active searches with their current states
                self.registry
                    .list_all_searches(connection_id)
                    .await
                    .map_err(McpError::Other)?
            }
            FsSearchAction::Kill => {
                // Gracefully cancel search and cleanup resources
                self.registry
                    .kill_search(connection_id, args.search)
                    .await
                    .map_err(McpError::Other)?
            }
            FsSearchAction::Read => {
                // Read current search state without re-executing
                let session = self
                    .registry
                    .find_or_create_search(connection_id, args.search)
                    .await
                    .map_err(McpError::Other)?;

                session.read_current_state().await.map_err(McpError::Other)?
            }
            FsSearchAction::Search => {
                // Execute search with timeout support
                let session = self
                    .registry
                    .find_or_create_search(connection_id, args.search)
                    .await
                    .map_err(McpError::Other)?;

                session
                    .execute_search_with_timeout(args.clone(), args.await_completion_ms, client_pwd)
                    .await
                    .map_err(McpError::Other)?
            }
        };

        // Extract summary from typed output
        let summary = output.output.clone();

        Ok(ToolResponse::new(summary, output))
    }
}
