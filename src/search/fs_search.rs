use kodegen_mcp_schema::filesystem::{FsSearchAction, FsSearchArgs, FsSearchPromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
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
    type PromptArgs = FsSearchPromptArgs;

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
    ) -> Result<Vec<Content>, McpError> {
        let connection_id = ctx.connection_id().unwrap_or("default");

        // Dispatch based on action
        let result = match args.action {
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
                
                session
                    .read_current_state()
                    .await
                    .map_err(McpError::Other)?
            }
            FsSearchAction::Search => {
                // Execute search with timeout support
                let session = self
                    .registry
                    .find_or_create_search(connection_id, args.search)
                    .await
                    .map_err(McpError::Other)?;

                session
                    .execute_search_with_timeout(args.clone(), args.await_completion_ms)
                    .await
                    .map_err(McpError::Other)?
            }
        };

        // Return both human-readable summary and JSON
        let summary = result["output"].as_str().unwrap_or("Search result");
        let json_str = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(vec![
            Content::text(summary.to_string()),
            Content::text(json_str),
        ])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "search_type".to_string(),
                title: Some("Search Focus".to_string()),
                description: Some(
                    "Which search type to focus examples on: 'content' (find code/text), 'filenames' (find files), \
                     or 'both' (show both, default). Use 'content' for most code search tasks, 'filenames' when \
                     you need to locate specific files by name."
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "complexity_level".to_string(),
                title: Some("Detail Level".to_string()),
                description: Some(
                    "Control explanation depth: 'beginner' (basic usage only), or 'advanced' (all parameters, \
                     edge cases, and patterns, default). Use 'beginner' for simple one-off searches, 'advanced' \
                     when building complex search strategies."
                        .to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "include_performance_tips".to_string(),
                title: Some("Performance Guidance".to_string()),
                description: Some(
                    "Whether to include max_depth and max_filesize optimization tips (default: true). \
                     Useful for monorepos and large codebases where search performance is critical."
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            // ========== FUNDAMENTAL CONCEPT ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What is fs_search and when should I use it?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "fs_search is a blazing-fast search tool (10-100x faster than grep) built on ripgrep. \
                     Use it to:\n\n\
                     1. Find specific files: \"locate all TypeScript config files\"\n\
                     2. Search code content: \"find where authentication is implemented\"\n\
                     3. Extract data: \"get all TODO comments\"\n\
                     4. Analyze patterns: \"find all console.log statements\"\n\n\
                     The tool returns results immediately in a single response (blocking search), making it \
                     perfect for most search tasks. It respects .gitignore automatically.",
                ),
            },
            
            // ========== CORE DISTINCTION ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "I'm confused about search_in='content' vs search_in='filenames'. What's the difference?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "This is the MOST IMPORTANT distinction in fs_search. They control WHERE the pattern is matched:\n\n\
                     **search_in='content'** (DEFAULT):\n\
                     - Searches INSIDE file contents\n\
                     - For finding code, text, comments, patterns\n\
                     - Example: Find all files containing 'error' → find code that handles errors\n\
                     - Example: Find all 'import React' statements → find React components\n\
                     - Example: Find 'TODO' comments → find incomplete work\n\n\
                     **search_in='filenames'**:\n\
                     - Searches file NAMES and PATHS only\n\
                     - For finding specific files or file types\n\
                     - Example: Find all package.json files → locate all packages\n\
                     - Example: Find files matching '*.config.js' → find all config files\n\
                     - Example: Find files named 'index' → find entry points\n\n\
                     **How to decide:**\n\
                     Ask: 'Am I looking for code/text CONTENT or trying to FIND FILES?'\n\
                     - Content search: \"find authentication logic\" → use search_in='content'\n\
                     - Find files: \"locate all test files\" → use search_in='filenames'",
                ),
            },
            
            // ========== RETURN MODES ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What do return_only modes do? I see 'matches', 'paths', and 'counts'.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "return_only controls WHAT DATA to return from results. It works INDEPENDENTLY of search_in:\n\n\
                     **return_only='matches'** (DEFAULT):\n\
                     - Return full details: file path, line number, match text, context lines\n\
                     - Most comprehensive results\n\
                     - Heaviest output (can be verbose)\n\
                     - Use when you need to see the actual matches\n\n\
                     **return_only='paths'**:\n\
                     - Return ONLY unique file paths (like `rg -l`)\n\
                     - Perfect for: \"which files contain this pattern?\"\n\
                     - Very compact output\n\
                     - Ignore line numbers and match text\n\n\
                     **return_only='counts'**:\n\
                     - Return match COUNT per file (like `rg -c`)\n\
                     - Perfect for: \"how many matches in each file?\"\n\
                     - Ultra-compact: just file path + count\n\n\
                     **CRITICAL: They combine independently:**\n\
                     You can combine ANY search_in with ANY return_only:\n\
                     - search_in='content' + return_only='paths' → which files contain this code?\n\
                     - search_in='content' + return_only='counts' → how many matches per file?\n\
                     - search_in='filenames' + return_only='matches' → show matching files with metadata\n\
                     - search_in='filenames' + return_only='paths' → just list matching filenames\n\n\
                     Example:\n\
                     - Find files: search_in='filenames', pattern='*.test.js', return_only='paths'\n\
                     - Count matches: search_in='content', pattern='error', return_only='counts'\n\
                     - Show all matches: search_in='content', pattern='TODO', return_only='matches'",
                ),
            },
            
            // ========== PATTERN MATCHING MODES ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I control how patterns are matched? I see options like literal_search and boundary_mode.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "fs_search has multiple ways to control pattern matching:\n\n\
                     **literal_search** (default: false):\n\
                     - false: Patterns are REGEX (default, most powerful)\n\
                     - true: Patterns are LITERAL STRINGS (exact match only)\n\n\
                     Use literal_search=true when:\n\
                     - Searching for exact code: literal_search=true, pattern=\"const x = 5\"\n\
                     - Pattern contains regex special chars: \"file(test).js\" would be literal\n\n\
                     **boundary_mode** (default: null/substring matching):\n\
                     - null: Pattern matches ANYWHERE (default, substring)\n\
                     - 'word': Match only WHOLE WORDS (boundary_mode='word')\n\
                     - 'line': Match complete LINES ONLY\n\n\
                     Use boundary_mode='word' when:\n\
                     - Search 'test' → matches 'test()' but NOT 'testing' or 'contest'\n\
                     - Search 'error' → matches 'error' but NOT 'ErrorHandler'\n\n\
                     Use boundary_mode='line' when:\n\
                     - Match complete lines only, useful for parsing\n\n\
                     **case_mode** (default: 'sensitive'):\n\
                     - 'sensitive': Case matters (TEST != test)\n\
                     - 'insensitive': Ignore case (case_mode='insensitive')\n\
                     - 'smart': Auto case-insensitive for all-lowercase patterns\n\n\
                     **multiline** (default: false):\n\
                     - false: Patterns match within single lines only\n\
                     - true: Enable multiline matching (multiline=true)\n\n\
                     Use multiline=true for structural patterns:\n\
                     ```\n\
                     pattern=\"function \\\\w+\\\\([^)]*\\\\) \\\\{[\\\\s\\\\S]*?\\\\}\"\n\
                     ```\n\
                     This spans multiple lines to find complete functions.\n\n\
                     **Common Combinations:**\n\
                     1. Find exact function: literal_search=true, pattern=\"function getName() {\"\n\
                     2. Find word 'bug': boundary_mode='word', pattern='bug'\n\
                     3. Case-insensitive: case_mode='insensitive', pattern='TODO'\n\
                     4. Structural pattern: multiline=true, case_mode='insensitive'",
                ),
            },
            
            // ========== PERFORMANCE ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I optimize fs_search performance in large codebases?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use these parameters to dramatically speed up searches (10-100x faster):\n\n\
                     **max_depth** (default: unlimited):\n\
                     - Limit directory traversal depth\n\
                     - max_depth=3 searches root + 3 levels only\n\
                     - Avoids deep node_modules, vendor/, target/ directories\n\n\
                     Common values:\n\
                     - max_depth=1: Root directory only + immediate children\n\
                     - max_depth=3: Good for avoiding dependencies in most projects\n\
                     - max_depth=4-5: Include moderate nesting like src/components/\n\n\
                     Example: Find TODO in source (skip node_modules):\n\
                     max_depth=3, pattern='TODO', search_in='content'\n\n\
                     **max_filesize** (default: unlimited):\n\
                     - Skip files larger than this size (in bytes)\n\
                     - Avoids huge minified bundles, lock files, generated code\n\n\
                     Common values:\n\
                     - 102400 (100KB): Ultra-fast, skip bundles\n\
                     - 1048576 (1MB): Recommended, skip minified files\n\
                     - 5242880 (5MB): Conservative, skip huge sources\n\n\
                     Example: Find code, skip minified:\n\
                     max_filesize=1048576, pattern='error', search_in='content'\n\n\
                     **file_pattern** (optional glob filter):\n\
                     - Only search specific file types\n\
                     - file_pattern='*.rs' searches only Rust files\n\
                     - file_pattern='*.{ts,tsx,js}' searches TypeScript/JavaScript\n\n\
                     Example: Find TypeScript errors:\n\
                     pattern='error', file_pattern='*.ts', search_in='content'\n\n\
                     **type filtering**:\n\
                     - Use file type names: type=['rust', 'json']\n\
                     - More efficient than glob patterns\n\n\
                     **BEST PRACTICE COMBINATION:**\n\
                     For large monorepos:\n\
                     max_depth=4, max_filesize=1048576, file_pattern='*.rs'\n\
                     This combination can provide 50-100x speedup.",
                ),
            },
            
            // ========== REAL-WORLD EXAMPLES ==========
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "Show me some real-world examples of using fs_search effectively.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Here are practical patterns you'll use frequently:\n\n\
                     **1. Find all TODO comments:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"TODO\", \"search_in\": \"content\", \
                     \"case_mode\": \"insensitive\"})\n\n\
                     **2. Locate config files:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"*.config.js\", \"search_in\": \"filenames\", \
                     \"return_only\": \"paths\"})\n\n\
                     **3. Find files with errors:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"error\", \"search_in\": \"content\", \
                     \"return_only\": \"paths\", \"file_pattern\": \"*.rs\"})\n\n\
                     **4. Count matches per file:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"import\", \"search_in\": \"content\", \
                     \"return_only\": \"counts\"})\n\n\
                     **5. Find exact code pattern:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"const x = 5\", \"search_in\": \"content\", \
                     \"literal_search\": true})\n\n\
                     **6. Extract version numbers:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"\\\\d+\\\\.\\\\d+\\\\.\\\\d+\", \
                     \"search_in\": \"content\", \"only_matching\": true, \"return_only\": \"matches\"})\n\n\
                     **7. Find test files:**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"*.test.ts\", \"search_in\": \"filenames\", \
                     \"return_only\": \"paths\"})\n\n\
                     **8. Find authentication code (optimized):**\n\
                     fs_search({\"path\": \".\", \"pattern\": \"authentication|auth|login\", \
                     \"search_in\": \"content\", \"max_depth\": 4, \"max_filesize\": 1048576, \
                     \"file_pattern\": \"*.ts\"})\n\n\
                     **Key Patterns:**\n\
                     - Use return_only='paths' when you only need file names\n\
                     - Use return_only='counts' to understand distribution\n\
                     - Always use max_depth and max_filesize in large projects\n\
                     - Combine search_in='filenames' with return_only='paths' for pure file listing\n\
                     - Use file_pattern to focus on relevant files (huge performance gain)",
                ),
            },
        ])
    }
}
