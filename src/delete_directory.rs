use crate::validate_path;
use kodegen_mcp_schema::filesystem::{FsDeleteDirectoryArgs, FsDeleteDirectoryPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use tokio::fs;

#[derive(Clone)]
pub struct DeleteDirectoryTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl DeleteDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

impl Tool for DeleteDirectoryTool {
    type Args = FsDeleteDirectoryArgs;
    type PromptArgs = FsDeleteDirectoryPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_DELETE_DIRECTORY
    }

    fn description() -> &'static str {
        "Delete a directory and all its contents recursively. This operation is permanent and \
         cannot be undone. Requires recursive=true to confirm deletion. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Permanently deletes data recursively
    }

    fn idempotent() -> bool {
        false // Deleting twice will fail
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        // Safety check: require explicit recursive flag
        if !args.recursive {
            return Err(McpError::InvalidArguments(
                "Must set recursive=true to delete a directory and its contents".to_string(),
            ));
        }

        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        // Check directory type (errors propagate naturally)
        let metadata = tokio::fs::metadata(&valid_path).await?;

        if !metadata.is_dir() {
            return Err(McpError::InvalidArguments(
                "Path is not a directory. Use delete_file to remove files.".to_string(),
            ));
        }

        fs::remove_dir_all(&valid_path).await?;

        let mut contents = Vec::new();

        // Human summary
        let summary = format!("✓ Deleted directory {} (recursive)", valid_path.display());
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "recursive": true
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
                content: PromptMessageContent::text("How do I delete directories?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The delete_directory tool recursively deletes directories:\n\n\
                     Usage: delete_directory({\"path\": \"/path/to/dir\", \"recursive\": true})\n\n\
                     Safety features:\n\
                     - Requires recursive=true flag to prevent accidental deletion\n\
                     - Only deletes directories, not individual files\n\
                     - Validates path exists and is actually a directory\n\
                     - Validates path is within allowed directories\n\
                     - Deletes ALL contents recursively (files and subdirectories)\n\n\
                     IMPORTANT: This operation is permanent and cannot be undone!\n\
                     All files and subdirectories will be permanently deleted.\n\n\
                     To delete individual files, use delete_file instead.",
                ),
            },
        ])
    }
}
