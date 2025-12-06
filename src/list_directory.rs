use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{DirectoryEntry, FsListDirectoryArgs, FsListDirectoryOutput, ListDirectoryPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use log::warn;
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ListDirectoryTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl ListDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ListDirectoryTool {
    type Args = FsListDirectoryArgs;
    type Prompts = ListDirectoryPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_LIST_DIRECTORY
    }

    fn description() -> &'static str {
        "List all files and directories in a specified path. Returns entries prefixed with \
         [DIR] or [FILE] to distinguish types. Supports filtering hidden files. \
         Automatically validates paths."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager, ctx.pwd()).await?;

        let mut read_entries = fs::read_dir(&valid_path).await?;
        let mut entries = Vec::new();
        let mut dir_count = 0;
        let mut file_count = 0;

        while let Some(entry) = read_entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if requested
            if !args.include_hidden && name.starts_with('.') {
                continue;
            }

            // Gracefully handle file_type errors - log and skip problematic entries
            let is_dir = match entry.file_type().await {
                Ok(ft) => ft.is_dir(),
                Err(e) => {
                    warn!(
                        "Skipping entry '{}' in '{}': {}",
                        name,
                        valid_path.display(),
                        e
                    );
                    continue;
                }
            };

            if is_dir {
                dir_count += 1;
            } else {
                file_count += 1;
            }

            entries.push(DirectoryEntry {
                name,
                is_directory: is_dir,
                size_bytes: None,
            });
        }

        // Sort for consistent output
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        // Human summary
        let total = entries.len();
        let display_path = shorten_path_for_display(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[36m󰉋 Listed directory: {}\x1b[0m\n 󰄵 Contents: {} items ({} dirs · {} files)",
            display_path,
            total,
            dir_count,
            file_count
        );

        Ok(ToolResponse::new(summary, FsListDirectoryOutput {
            success: true,
            path: valid_path.to_string_lossy().to_string(),
            total_entries: total,
            directories: dir_count,
            files: file_count,
            entries,
        }))
    }
}
