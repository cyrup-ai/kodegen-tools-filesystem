use crate::validate_path;
use kodegen_mcp_schema::filesystem::{FsMoveFileArgs, FsMoveFilePromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
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
    type PromptArgs = FsMoveFilePromptArgs;

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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let source_path = validate_path(&args.source, &self.config_manager).await?;
        let dest_path = validate_path(&args.destination, &self.config_manager).await?;

        fs::rename(&source_path, &dest_path).await?;

        let mut contents = Vec::new();

        // Human summary
        let summary = format!(
            "\x1b[34m󰉐 Moved file/directory\x1b[0m\n\
             󰜱 From: {}\n\
             󰜱 To:   {}",
            source_path.display(),
            dest_path.display()
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "source": source_path.to_string_lossy(),
            "destination": dest_path.to_string_lossy()
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
                content: PromptMessageContent::text("How do I move or rename files?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The move_file tool moves or renames files and directories:\n\n\
                     1. Rename: move_file({\"source\": \"/path/old.txt\", \"destination\": \"/path/new.txt\"})\n\
                     2. Move: move_file({\"source\": \"/path/file.txt\", \"destination\": \"/other/file.txt\"})\n\
                     3. Move directory: move_file({\"source\": \"/path/dir\", \"destination\": \"/other/dir\"})\n\n\
                     Important notes:\n\
                     - Both source and destination paths are validated\n\
                     - Source must exist or the operation fails\n\
                     - If destination exists, it may be overwritten (OS-dependent)\n\
                     - Moving a directory moves all its contents\n\
                     - This operation is atomic on most filesystems",
                ),
            },
        ])
    }
}
