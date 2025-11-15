use super::SearchManager;
use kodegen_mcp_schema::filesystem::{FsStopSearchArgs, FsStopSearchPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct StopSearchTool {
    manager: Arc<SearchManager>,
}

impl StopSearchTool {
    #[must_use]
    pub fn new(manager: Arc<SearchManager>) -> Self {
        Self { manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for StopSearchTool {
    type Args = FsStopSearchArgs;
    type PromptArgs = FsStopSearchPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_STOP_SEARCH
    }

    fn description() -> &'static str {
        "Stop an active search session.\n\n\
         Stops the background search process gracefully. Use this when you've found \
         what you need or if a search is taking too long. Similar to force_terminate \
         for terminal processes.\n\n\
         The search will still be available for reading final results until it's \
         automatically cleaned up after 5 minutes."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let success = self.manager.terminate_search(&args.session_id).await?;

        let mut contents = Vec::new();

        if success {
            // Content 1: Human-readable summary
            let summary = format!(
                "✓ Stopped search session: {}\n\nResults remain available for reading.",
                args.session_id
            );
            contents.push(Content::text(summary));

            // Content 2: JSON metadata
            let metadata = json!({
                "success": true,
                "session_id": args.session_id,
                "message": format!(
                    "Search session {} terminated successfully. Results remain available for reading.",
                    args.session_id
                )
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        } else {
            // Content 1: Human-readable summary (failure case)
            let summary = format!(
                "⚠️  Search session not found: {}\n\nSession may have already completed or been cleaned up.",
                args.session_id
            );
            contents.push(Content::text(summary));

            // Content 2: JSON metadata (failure case)
            let metadata = json!({
                "success": false,
                "session_id": args.session_id,
                "message": format!("Search session {} not found or already completed.", args.session_id)
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        }

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I stop a search?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The stop_search tool terminates active search sessions:\n\n\
                     Basic usage:\n\
                     stop_search({\"session_id\": \"search_123_1234567890\"})\n\n\
                     How it works:\n\
                     1. Takes a session_id from start_search\n\
                     2. Kills the ripgrep process (SIGKILL)\n\
                     3. Marks the search as complete\n\
                     4. Keeps results available for reading\n\n\
                     Typical workflow:\n\
                     1. Start: start_search({\"path\": \"/large/directory\", \"pattern\": \"TODO\"})\n\
                        Returns: {\"session_id\": \"search_123_1234567890\"}\n\
                     2. Read: get_search_results({\"session_id\": \"search_123_1234567890\"})\n\
                        See initial results, realize you found what you need\n\
                     3. Stop: stop_search({\"session_id\": \"search_123_1234567890\"})\n\
                        Terminates the search early\n\
                     4. Read: get_search_results({\"session_id\": \"search_123_1234567890\"})\n\
                        Can still read final results\n\n\
                     When to use:\n\
                     - Found what you need in first few results\n\
                     - Search is taking too long on large codebase\n\
                     - Want to refine search with different pattern\n\
                     - Accidentally started wrong search\n\n\
                     Similar to force_terminate for terminal processes:\n\
                     - Both stop background processes\n\
                     - Both keep results available\n\
                     - Both return success/not found\n\n\
                     After termination:\n\
                     - Session remains in memory for 5 minutes\n\
                     - Can still call get_search_results\n\
                     - Automatic cleanup removes old sessions",
                ),
            },
        ])
    }
}
