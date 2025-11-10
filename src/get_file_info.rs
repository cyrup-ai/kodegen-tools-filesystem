use crate::validate_path;
use kodegen_mcp_schema::filesystem::{GetFileInfoArgs, GetFileInfoPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::time::SystemTime;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Count lines in a file using streaming with O(1) memory
///
/// Uses tokio's BufReader to stream through file line-by-line
/// without loading entire contents into memory.
///
/// Memory usage: ~8KB buffer regardless of file size
async fn count_lines_streaming(path: &std::path::Path) -> Result<usize, McpError> {
    let file = tokio::fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines_stream = reader.lines();
    let mut count = 0;

    while lines_stream.next_line().await?.is_some() {
        count += 1;
    }

    Ok(count)
}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct GetFileInfoTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl GetFileInfoTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetFileInfoTool {
    type Args = GetFileInfoArgs;
    type PromptArgs = GetFileInfoPromptArgs;

    fn name() -> &'static str {
        "get_file_info"
    }

    fn description() -> &'static str {
        "Retrieve detailed metadata about a file or directory including size, creation time, \
         last modified time, permissions, type, and line count (for text files under 10MB). \
         Automatically validates paths."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;
        let stats = fs::metadata(&valid_path).await?;

        let now = SystemTime::now();
        let modified_secs_ago = match stats.modified() {
            Ok(m) => now.duration_since(m).unwrap_or_default().as_secs(),
            Err(_) => 0,
        };

        let mut info = json!({
            "path": valid_path.to_string_lossy(),
            "size": stats.len(),
            "created": format!("{:?}", stats.created().ok()),
            "modified_secs_ago": modified_secs_ago,
            "accessed": format!("{:?}", stats.accessed().ok()),
            "is_directory": stats.is_dir(),
            "is_file": stats.is_file(),
        });

        // Platform-specific permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            info["permissions"] = json!(format!("{:o}", stats.permissions().mode() & 0o777));
        }

        #[cfg(windows)]
        {
            info["readonly"] = json!(stats.permissions().readonly());
        }

        // For text files under 10MB, calculate line count using streaming
        if stats.is_file() && stats.len() < 10 * 1024 * 1024 {
            match count_lines_streaming(&valid_path).await {
                Ok(line_count) => {
                    info["line_count"] = json!(line_count);
                    if line_count > 0 {
                        info["last_line"] = json!(line_count - 1); // zero-indexed
                        info["append_position"] = json!(line_count); // for appending
                    }
                }
                Err(_) => {
                    // Not a text file or encoding error - skip line count silently
                }
            }
        }

        Ok(info)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I get file metadata?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The get_file_info tool provides comprehensive file/directory metadata:\n\n\
                     Usage: get_file_info({\"path\": \"/path/to/file.txt\"})\n\n\
                     Returns:\n\
                     - size: File size in bytes\n\
                     - created: Creation timestamp\n\
                     - modified_secs_ago: Seconds since last modification\n\
                     - accessed: Last access time\n\
                     - is_directory: Whether path is a directory\n\
                     - is_file: Whether path is a file\n\
                     - permissions: Unix permissions in octal (Unix only)\n\
                     - readonly: Read-only flag (Windows only)\n\
                     - line_count: Number of lines (text files < 10MB only)\n\
                     - last_line: Zero-indexed last line number\n\
                     - append_position: Line number for appending\n\n\
                     The tool automatically:\n\
                     - Validates and normalizes paths\n\
                     - Handles platform-specific permission formats\n\
                     - Efficiently calculates line counts for small text files",
                ),
            },
        ])
    }
}
