use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FsCreateDirectoryArgs, FsCreateDirectoryOutput, CreateDirectoryPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct CreateDirectoryTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl CreateDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for CreateDirectoryTool {
    type Args = FsCreateDirectoryArgs;
    type Prompts = CreateDirectoryPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_CREATE_DIRECTORY
    }

    fn description() -> &'static str {
        "Create a new directory or ensure a directory exists. Can create multiple nested \
         directories in one operation. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false // Creates only, doesn't delete
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager, ctx.pwd()).await?;

        fs::create_dir_all(&valid_path).await?;

        // Human summary
        let display_path = shorten_path_for_display(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[32mCreated directory: {}\x1b[0m\n\
             Status: Directory ready (idempotent)",
            display_path
        );

        Ok(ToolResponse::new(summary, FsCreateDirectoryOutput {
            success: true,
            path: valid_path.to_string_lossy().to_string(),
            created: true,
            message: "Directory created successfully".to_string(),
        }))
    }
}
