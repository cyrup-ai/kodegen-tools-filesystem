use crate::validate_path;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FsGetFileInfoArgs, FsGetFileInfoOutput, GetFileInfoPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
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
    type Args = FsGetFileInfoArgs;
    type Prompts = GetFileInfoPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_GET_FILE_INFO
    }

    fn description() -> &'static str {
        "Retrieve detailed metadata about a file or directory including size, creation time, \
         last modified time, permissions, type, and line count (for text files under 10MB). \
         Automatically validates paths."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager, ctx.pwd()).await?;
        let stats = fs::metadata(&valid_path).await?;

        let now = SystemTime::now();
        let modified_secs_ago = match stats.modified() {
            Ok(m) => now.duration_since(m).unwrap_or_default().as_secs(),
            Err(_) => 0,
        };

        // Format timestamps
        let created = stats.created().ok().map(|t| format!("{t:?}"));
        let modified = stats.modified().ok().map(|t| format!("{t:?}"));
        let accessed = stats.accessed().ok().map(|t| format!("{t:?}"));

        // Check for symlink
        let symlink_meta = fs::symlink_metadata(&valid_path).await?;
        let is_symlink = symlink_meta.file_type().is_symlink();

        // For text files under 10MB, calculate line count using streaming
        let mut line_count_opt = None;
        if stats.is_file() && stats.len() < 10 * 1024 * 1024
            && let Ok(lc) = count_lines_streaming(&valid_path).await {
            line_count_opt = Some(lc as u64);
        }

        // Platform-specific permissions string for display
        #[cfg(unix)]
        let perms_str = {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", stats.permissions().mode() & 0o777)
        };
        #[cfg(windows)]
        let perms_str = if stats.permissions().readonly() {
            "readonly".to_string()
        } else {
            "read-write".to_string()
        };

        // Human summary
        let type_str = if stats.is_dir() { "Directory" } else { "File" };
        let size_kb = stats.len() as f64 / 1024.0;
        let size_str = if size_kb < 1024.0 {
            format!("{size_kb:.1} KB")
        } else {
            format!("{:.1} MB", size_kb / 1024.0)
        };
        let time_str = if modified_secs_ago < 60 {
            format!("{modified_secs_ago} seconds ago")
        } else if modified_secs_ago < 3600 {
            format!("{} minutes ago", modified_secs_ago / 60)
        } else if modified_secs_ago < 86400 {
            format!("{} hours ago", modified_secs_ago / 3600)
        } else {
            format!("{} days ago", modified_secs_ago / 86400)
        };
        let line_count_str = line_count_opt.map_or(String::new(), |lc| format!("{lc} lines · "));
        let display_path = shorten_path_for_display(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[35m{type_str} Metadata: {display_path}\x1b[0m\n\
             Details: {size_str} · {line_count_str}Modified: {time_str} · Perms: {perms_str}"
        );

        Ok(ToolResponse::new(summary, FsGetFileInfoOutput {
            success: true,
            path: valid_path.to_string_lossy().to_string(),
            exists: true,
            is_file: stats.is_file(),
            is_directory: stats.is_dir(),
            is_symlink,
            size_bytes: Some(stats.len()),
            created,
            modified,
            accessed,
            line_count: line_count_opt,
        }))
    }
}
