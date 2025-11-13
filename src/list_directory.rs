use crate::validate_path;
use kodegen_mcp_schema::filesystem::{FsListDirectoryArgs, FsListDirectoryPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use log::warn;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
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
    type PromptArgs = FsListDirectoryPromptArgs;

    fn name() -> &'static str {
        "fs_list_directory"
    }

    fn description() -> &'static str {
        "List all files and directories in a specified path. Returns entries prefixed with \
         [DIR] or [FILE] to distinguish types. Supports filtering hidden files. \
         Automatically validates paths."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        let mut entries = fs::read_dir(&valid_path).await?;
        let mut items = Vec::new();
        let mut dir_count = 0;
        let mut file_count = 0;

        while let Some(entry) = entries.next_entry().await? {
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
                items.push(format!("[DIR]  {name}"));
                dir_count += 1;
            } else {
                items.push(format!("[FILE] {name}"));
                file_count += 1;
            }
        }

        // Sort for consistent output
        items.sort();

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let total = items.len();
        let truncated = total > 50;
        let display_items = if truncated {
            &items[..50]
        } else {
            &items[..]
        };

        let mut summary = format!(
            "📁 Listed {}\n\nContents ({} items):\n- {} directories\n- {} files\n\n{}",
            valid_path.display(),
            total,
            dir_count,
            file_count,
            display_items.join("\n")
        );

        if truncated {
            summary.push_str(&format!("\n... ({} more items not shown)", total - 50));
        }

        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "total": total,
            "directories": dir_count,
            "files": file_count,
            "entries": items,
            "truncated": truncated
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "show_advanced".to_string(),
            title: None,
            description: Some("Show advanced filtering options".to_string()),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I list directory contents?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_directory tool shows all files and directories:\n\n\
                     1. Basic usage: list_directory({\"path\": \"/path/to/dir\"})\n\
                     2. Include hidden files: list_directory({\"path\": \"/path/to/dir\", \"include_hidden\": true})\n\n\
                     Output format:\n\
                     - Directories are prefixed with [DIR]\n\
                     - Files are prefixed with [FILE]\n\
                     - Results are sorted alphabetically\n\n\
                     The tool automatically:\n\
                     - Validates the directory path exists\n\
                     - Filters hidden files by default (unless include_hidden=true)\n\
                     - Provides counts of directories and files\n\
                     - Handles permission errors gracefully",
                ),
            },
        ])
    }
}
