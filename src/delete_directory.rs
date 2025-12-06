use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FsDeleteDirectoryArgs, FsDeleteDirectoryOutput, DeleteDirectoryPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
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
    type Prompts = DeleteDirectoryPrompts;

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

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        // Safety check: require explicit recursive flag
        if !args.recursive {
            return Err(McpError::InvalidArguments(
                "Must set recursive=true to delete a directory and its contents".to_string(),
            ));
        }

        let valid_path = validate_path(&args.path, &self.config_manager, ctx.pwd()).await?;

        // Check directory type (errors propagate naturally)
        let metadata = tokio::fs::metadata(&valid_path).await?;

        if !metadata.is_dir() {
            return Err(McpError::InvalidArguments(
                "Path is not a directory. Use delete_file to remove files.".to_string(),
            ));
        }

        fs::remove_dir_all(&valid_path).await?;

        // Human summary
        let display_path = shorten_path_for_display(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[31m󰆴 Deleted directory (recursive)\x1b[0m\n\
             󰉋 Removed: {}\n\
             󰚽 Permanent: All contents deleted",
            display_path
        );

        Ok(ToolResponse::new(summary, FsDeleteDirectoryOutput {
            success: true,
            path: valid_path.to_string_lossy().to_string(),
            message: "Directory and all contents deleted successfully".to_string(),
        }))
    }
}
