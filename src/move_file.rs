use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FsMoveFileArgs, FsMoveFileOutput, MoveFilePrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use tokio::fs;

#[derive(Clone)]
pub struct MoveFileTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl MoveFileTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

impl Tool for MoveFileTool {
    type Args = FsMoveFileArgs;
    type Prompts = MoveFilePrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_MOVE_FILE
    }

    fn description() -> &'static str {
        "Move or rename files and directories. Can move files between directories and rename \
         them in a single operation. Both source and destination must be within allowed directories."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Can overwrite destination
    }

    fn idempotent() -> bool {
        false // Moving twice would fail (source no longer exists)
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let client_pwd = ctx.pwd();
        let source_path = validate_path(&args.source, &self.config_manager, client_pwd).await?;
        let dest_path = validate_path(&args.destination, &self.config_manager, client_pwd).await?;

        fs::rename(&source_path, &dest_path).await?;

        // Human summary
        let display_source = shorten_path_for_display(&source_path, ctx.git_root());
        let display_dest = shorten_path_for_display(&dest_path, ctx.git_root());
        let summary = format!(
            "\x1b[34m󰉐 Moved file/directory\x1b[0m\n\
             󰜱 From: {}\n\
             󰜱 To:   {}",
            display_source,
            display_dest
        );

        Ok(ToolResponse::new(summary, FsMoveFileOutput {
            success: true,
            source: source_path.to_string_lossy().to_string(),
            destination: dest_path.to_string_lossy().to_string(),
            message: "File/directory moved successfully".to_string(),
        }))
    }
}
