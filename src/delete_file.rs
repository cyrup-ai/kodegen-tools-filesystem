use crate::validate_path;
use kodegen_mcp_schema::filesystem::{FsDeleteFileArgs, FsDeleteFilePromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use tokio::fs;

#[derive(Clone)]
pub struct DeleteFileTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl DeleteFileTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

impl Tool for DeleteFileTool {
    type Args = FsDeleteFileArgs;
    type PromptArgs = FsDeleteFilePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_DELETE_FILE
    }

    fn description() -> &'static str {
        "Delete a file from the filesystem. This operation is permanent and cannot be undone. \
         Only deletes files, not directories. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Permanently deletes data
    }

    fn idempotent() -> bool {
        false // Deleting twice will fail (file no longer exists)
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        // Check file type (errors propagate naturally)
        let metadata = tokio::fs::metadata(&valid_path).await?;

        if !metadata.is_file() {
            return Err(McpError::InvalidArguments(
                "Path is not a file. Use delete_directory to remove directories.".to_string(),
            ));
        }

        fs::remove_file(&valid_path).await?;

        let mut contents = Vec::new();

        // Human summary
        let summary = format!(
            "\x1b[31m󰆴 Deleted file: {}\x1b[0m\n\
             󰚽 Permanent: File removed from filesystem",
            valid_path.display()
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "path": valid_path.to_string_lossy()
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "context".to_string(),
            title: None,
            description: Some(
                "Optional context for examples (e.g., 'ci_cleanup', 'workflow', 'safety_patterns')"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use delete_file safely and when should I use it?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The delete_file tool permanently removes individual files:\n\n\
                     Basic usage: delete_file({\"path\": \"/path/to/file.txt\"})\n\n\
                     Key safety features:\n\
                     - Only deletes files (not directories - use delete_directory for that)\n\
                     - Validates path exists and is a regular file before deletion\n\
                     - Validates path is within allowed directories\n\
                     - Returns clear errors if file doesn't exist or is a directory\n\
                     - Marked as destructive and non-idempotent (deleting twice fails)\n\n\
                     Common use cases:\n\
                     1. CI/CD cleanup: delete temporary build artifacts\n\
                     2. Workflow automation: remove processed files\n\
                     3. Cache management: delete stale or expired files\n\
                     4. File rotation: delete old log or backup files\n\
                     5. Build output: remove intermediate compilation outputs\n\n\
                     Important patterns:\n\
                     - Always verify the path is correct before deletion\n\
                     - Use fs_search or fs_list_directory to verify file exists first\n\
                     - For batch deletions, iterate with delete_file (don't use delete_directory)\n\
                     - To delete directories and contents, use delete_directory with recursive=true\n\
                     - Handle non-existent files gracefully in automation\n\n\
                     CRITICAL: This operation is permanent and cannot be undone!\n\
                     Once deleted, file recovery requires filesystem utilities outside this tool.",
                ),
            },
        ])
    }
}
