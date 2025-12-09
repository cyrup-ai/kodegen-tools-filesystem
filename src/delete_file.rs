use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FsDeleteFileArgs, FsDeleteFileOutput, DeleteFilePrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
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
    type Prompts = DeleteFilePrompts;

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

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager, ctx.pwd()).await?;

        // Check file type (errors propagate naturally)
        let metadata = tokio::fs::metadata(&valid_path).await?;

        if !metadata.is_file() {
            return Err(McpError::InvalidArguments(
                "Path is not a file. Use delete_directory to remove directories.".to_string(),
            ));
        }

        fs::remove_file(&valid_path).await?;

        // Human summary
        let display_path = shorten_path_for_display(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[31mDeleted file: {}\x1b[0m\n\
             Permanent: File removed from filesystem",
            display_path
        );

        Ok(ToolResponse::new(summary, FsDeleteFileOutput {
            success: true,
            path: valid_path.to_string_lossy().to_string(),
            message: "File deleted successfully".to_string(),
        }))
    }
}
