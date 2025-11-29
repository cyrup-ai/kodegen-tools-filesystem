use crate::{validate_path, display_path_relative_to_git_root};
use kodegen_mcp_schema::filesystem::{FsWriteFileArgs, FsWriteFilePromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct WriteFileTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl WriteFileTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for WriteFileTool {
    type Args = FsWriteFileArgs;
    type PromptArgs = FsWriteFilePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_WRITE_FILE
    }

    fn description() -> &'static str {
        "Write or append to file contents. Supports two modes: 'rewrite' (overwrite entire file) \
         and 'append' (add to end of file). Automatically validates paths and creates parent \
         directories if needed."
    }

    fn read_only() -> bool {
        false // Modifies filesystem
    }

    fn destructive() -> bool {
        true // Can overwrite files
    }

    fn idempotent() -> bool {
        false // Each write changes the file
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        // Create parent directories if needed
        if let Some(parent) = valid_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Get file metadata for response
        let extension = valid_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content_bytes = args.content.len();
        let line_count = args.content.lines().count();

        // Perform write operation
        if args.mode == "append" {
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&valid_path)
                .await?;
            file.write_all(args.content.as_bytes()).await?;
        } else {
            fs::write(&valid_path, args.content).await?;
        }

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let verb = if args.mode == "append" {
            "Appended"
        } else {
            "Wrote"
        };
        let display_path = display_path_relative_to_git_root(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[32m󰏫 {} file: {}\x1b[0m\n\
             󰄴 Written: {} bytes ({} lines) · Mode: {}",
            verb,
            display_path,
            content_bytes,
            line_count,
            args.mode
        );
        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "mode": args.mode,
            "bytes_written": content_bytes,
            "lines_written": line_count,
            "file_extension": extension
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "example_type".to_string(),
            title: None,
            description: Some("Type of example to show (e.g., 'append', 'overwrite')".to_string()),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use fs_write_file to safely write and append to files?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The fs_write_file tool supports two modes:\n\n\
                     1. Rewrite mode (default): fs_write_file({\"path\": \"file.txt\", \"content\": \"new content\"})\n\
                     2. Append mode: fs_write_file({\"path\": \"file.txt\", \"content\": \"more content\", \"mode\": \"append\"})\n\n\
                     The tool automatically:\n\
                     - Validates and normalizes file paths\n\
                     - Creates parent directories if needed\n\
                     - Handles file permissions\n\
                     - Creates the file if it doesn't exist (both modes)\n\n\
                     Safety notes:\n\
                     - Rewrite mode overwrites the entire file\n\
                     - Append mode safely adds to the end\n\
                     - Path validation prevents writing outside allowed directories",
                ),
            },
        ])
    }
}
