use super::SearchManager;
use kodegen_mcp_schema::filesystem::{FsListSearchesArgs, FsListSearchesPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ListSearchesTool {
    search_manager: Arc<SearchManager>,
}

impl ListSearchesTool {
    #[must_use]
    pub fn new(search_manager: Arc<SearchManager>) -> Self {
        Self { search_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ListSearchesTool {
    type Args = FsListSearchesArgs;
    type PromptArgs = FsListSearchesPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_LIST_SEARCHES
    }

    fn description() -> &'static str {
        "List all active searches.\n\n\
         Shows search IDs, search types, patterns, status, and runtime.\n\
         Similar to list_sessions for terminal processes. Useful for managing\n\
         multiple concurrent searches."
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

    async fn execute(&self, _args: Self::Args) -> Result<Vec<Content>, McpError> {
        let sessions = self.search_manager.list_active_sessions().await;

        let mut contents = Vec::new();

        // Content 1: Human-readable summary with ANSI colors and icons
        let count = sessions.len();
        let summary = if sessions.is_empty() {
            "\x1b[36m󰅍 Active searches\x1b[0m\n 󰓎 Sessions: 0 active · No searches running".to_string()
        } else {
            format!(
                "\x1b[36m󰅍 Active searches\x1b[0m\n 󰓎 Sessions: {} active",
                count
            )
        };
        contents.push(Content::text(summary));

        // Content 2: JSON metadata
        let metadata = json!({
            "success": true,
            "sessions": sessions,
            "count": count,
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
                content: PromptMessageContent::text("How do I see all my running searches?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_searches tool shows all active search sessions:\n\n\
                     Basic usage:\n\
                     list_searches({})\n\n\
                     Returns:\n\
                     {\n\
                       \"sessions\": [\n\
                         {\n\
                           \"id\": \"search_1_1234567890\",\n\
                           \"search_type\": \"content\",\n\
                           \"pattern\": \"TODO\",\n\
                           \"is_complete\": false,\n\
                           \"is_error\": false,\n\
                           \"runtime_ms\": 5430,\n\
                           \"total_results\": 127\n\
                         },\n\
                         {\n\
                           \"id\": \"search_2_1234567891\",\n\
                           \"search_type\": \"files\",\n\
                           \"pattern\": \"*.rs\",\n\
                           \"is_complete\": true,\n\
                           \"is_error\": false,\n\
                           \"runtime_ms\": 2100,\n\
                           \"total_results\": 45\n\
                         }\n\
                       ],\n\
                       \"count\": 2\n\
                     }\n\n\
                     Understanding the output:\n\
                     - id: Session ID for use with get_search_results or stop_search\n\
                     - search_type: \\\"files\\\" (filename search) or \\\"content\\\" (text search)\n\
                     - pattern: The search pattern being used\n\
                     - is_complete: true = finished, false = still running\n\
                     - is_error: true = encountered errors\n\
                     - runtime_ms: How long the search has been running\n\
                     - total_results: Number of results found so far\n\n\
                     Common workflows:\n\
                     1. Check what's searching: list_searches() → See all session IDs\n\
                     2. Get results from specific search: get_search_results({\\\"session_id\\\": \\\"search_1_...\\\"}) \n\
                     3. Stop unwanted search: stop_search({\\\"session_id\\\": \\\"search_1_...\\\"})\n\n\
                     When to use:\n\
                     - Lost track of running searches\n\
                     - Want to monitor all active search operations\n\
                     - Need session IDs for other operations\n\
                     - Checking if long-running search is still active\n\n\
                     Best practices:\n\
                     - Call periodically to monitor long-running searches\n\
                     - Use with get_search_results to process multiple searches\n\
                     - Check before starting new searches to avoid overload\n\n\
                     Example multi-search management:\n\
                     1. start_search({\\\"pattern\\\": \\\"TODO\\\", \\\"search_type\\\": \\\"content\\\"}) → search_1\n\
                     2. start_search({\\\"pattern\\\": \\\"*.rs\\\", \\\"search_type\\\": \\\"files\\\"}) → search_2\n\
                     3. list_searches() → [{id: search_1, ...}, {id: search_2, ...}]\n\
                     4. get_search_results({\\\"session_id\\\": \\\"search_1\\\"}) → Get TODO results\n\
                     5. stop_search({\\\"session_id\\\": \\\"search_2\\\"}) → Cancel file search if needed",
                ),
            },
        ])
    }
}
