use crate::validate_path;
use kodegen_mcp_schema::filesystem::{FsGetFileInfoArgs, FsGetFileInfoPromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
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
    type PromptArgs = FsGetFileInfoPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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
        let perms_str;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms_str = format!("{:o}", stats.permissions().mode() & 0o777);
            info["permissions"] = json!(&perms_str);
        }

        #[cfg(windows)]
        {
            perms_str = if stats.permissions().readonly() {
                "readonly".to_string()
            } else {
                "read-write".to_string()
            };
            info["readonly"] = json!(stats.permissions().readonly());
        }

        // For text files under 10MB, calculate line count using streaming
        let mut line_count_opt = None;
        if stats.is_file() && stats.len() < 10 * 1024 * 1024 {
            match count_lines_streaming(&valid_path).await {
                Ok(line_count) => {
                    line_count_opt = Some(line_count);
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

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let type_str = if stats.is_dir() {
            "Directory"
        } else {
            "File"
        };

        let size_kb = stats.len() as f64 / 1024.0;
        let size_str = if size_kb < 1024.0 {
            format!("{:.1} KB", size_kb)
        } else {
            format!("{:.1} MB", size_kb / 1024.0)
        };

        let time_str = if modified_secs_ago < 60 {
            format!("{} seconds ago", modified_secs_ago)
        } else if modified_secs_ago < 3600 {
            format!("{} minutes ago", modified_secs_ago / 60)
        } else if modified_secs_ago < 86400 {
            format!("{} hours ago", modified_secs_ago / 3600)
        } else {
            format!("{} days ago", modified_secs_ago / 86400)
        };

        // Line count string (if available)
        let line_count_str = line_count_opt.map_or(String::new(), |lc| format!("{} lines · ", lc));

        // Build compact two-line format with magenta color on line 1 only
        let summary = format!(
            "\x1b[35m󰙅 {} Metadata: {}\x1b[0m\n\
             󰘖 Details: {} · {}Modified: {} · Perms: {}",
            type_str,
            valid_path.display(),
            size_str,
            line_count_str,
            time_str,
            perms_str
        );

        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        info["success"] = json!(true);
        let json_str = serde_json::to_string_pretty(&info)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "focus_area".to_string(),
            title: None,
            description: Some(
                "Optional focus area for examples: 'permissions' (Unix vs Windows), 'timestamps' (temporal metadata), \
                 'sizes' (byte calculations), 'line_counts' (text file analysis), 'platform_differences' (cross-platform behavior), \
                 or 'all' (comprehensive overview)".to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("What does the fs_get_file_info tool do and when should I use it?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The fs_get_file_info tool retrieves comprehensive metadata about any file or directory without loading \
                     its contents. Use it to:\n\
                     - Determine if a path is a file or directory\n\
                     - Get file size in bytes for budget/performance decisions\n\
                     - Understand modification times for cache invalidation or sync workflows\n\
                     - Extract line count for text files (essential for chunking strategies)\n\
                     - Inspect permissions before read/write operations\n\n\
                     Basic usage: fs_get_file_info({\"path\": \"/path/to/file.txt\"})\n\n\
                     Returns two outputs:\n\
                     1. Human-readable summary with formatted size and relative times\n\
                     2. Structured JSON with all metadata fields",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("Walk me through the JSON response fields and what each means."),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Core metadata fields returned:\n\n\
                     IDENTIFICATION:\n\
                     - path: Normalized absolute path after validation\n\
                     - is_file: Boolean - true if it's a regular file\n\
                     - is_directory: Boolean - true if it's a directory\n\n\
                     SIZE INFORMATION:\n\
                     - size: Raw byte count (use to decide if file fits in memory)\n\n\
                     TIMESTAMPS (all in system time format):\n\
                     - created: Creation time (when file/dir was created)\n\
                     - accessed: Last read time (updates may be disabled on some filesystems)\n\
                     - modified_secs_ago: Seconds since last write (useful for relative time checks)\n\n\
                     TEXT FILE ANALYSIS (text files < 10MB only):\n\
                     - line_count: Total number of lines in file\n\
                     - last_line: Zero-indexed number of final line (useful for range operations)\n\
                     - append_position: Line number to append at (equals line_count)\n\n\
                     PERMISSIONS (platform-specific):\n\
                     - Unix: 'permissions' field contains 3-digit octal (e.g., \"644\" for rw-r--r--)\n\
                     - Windows: 'readonly' boolean flag (true means read-only)",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do permissions differ between Unix and Windows?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "UNIX PERMISSIONS (octal notation):\n\
                     The 'permissions' field shows 3 digits: owner, group, others\n\
                     Each digit: 4=read, 2=write, 1=execute (sum them)\n\
                     Examples:\n\
                     - 755 = rwxr-xr-x (executable, others can read/execute)\n\
                     - 644 = rw-r--r-- (readable by all, writable by owner only)\n\
                     - 700 = rwx------ (owner only, no group/other access)\n\n\
                     WINDOWS PERMISSIONS:\n\
                     The tool returns a simple 'readonly' boolean:\n\
                     - true = file/directory is read-only (use fs_edit_block not fs_write_file)\n\
                     - false = file/directory is writable\n\n\
                     CROSS-PLATFORM TIP:\n\
                     If targeting both platforms, check 'readonly' on Windows and the last digit \
                     of Unix 'permissions' to determine write access. On Unix, also consider if \
                     you're the owner (critical for write decisions).",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("What are the limitations and edge cases I should know?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "IMPORTANT LIMITATIONS:\n\n\
                     1. LINE COUNTING:\n\
                     - Only available for files < 10MB (uses streaming for memory efficiency)\n\
                     - For larger files, you must estimate or use fs_read_file with offset/length\n\
                     - Binary files may return no line count (encoding errors are silently skipped)\n\n\
                     2. TIMESTAMPS:\n\
                     - 'created' may not be reliable on all filesystems (shows as ??? if unavailable)\n\
                     - 'accessed' may not update depending on filesystem mount options (noatime)\n\
                     - Modified time is most reliable across platforms\n\n\
                     3. SYMLINKS:\n\
                     - The tool follows symlinks and returns metadata for target file\n\
                     - To detect if something is a symlink, use fs_read_file or fs_list_directory\n\n\
                     4. PATH VALIDATION:\n\
                     - Paths must be within allowed directories (config-based restrictions)\n\
                     - Tilde (~) is expanded to home directory\n\
                     - Relative paths are resolved relative to cwd\n\n\
                     COMMON MISTAKES:\n\
                     - Assuming created/accessed times are always present (they can be empty)\n\
                     - Relying on line count for very large files (get file size first, check < 10MB)\n\
                     - Forgetting that Unix permissions require owner check for write decisions",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("Show me practical usage patterns and examples."),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "PRACTICAL PATTERNS:\n\n\
                     PATTERN 1: Safe file reading strategy\n\
                     ```\n\
                     1. Call fs_get_file_info({\"path\": \"file.txt\"})\n\
                     2. If size > 10MB: Use fs_read_file with offset/length\n\
                     3. If size <= 10MB and line_count exists: Safe to read fully\n\
                     4. If line_count missing: Binary file - use base64 from fs_read_file\n\
                     ```\n\n\
                     PATTERN 2: Check if you can write\n\
                     ```\n\
                     1. Call fs_get_file_info({\"path\": \"file.txt\"})\n\
                     2. Unix: Parse 'permissions' last digit (1=executable, 2=write, 4=read)\n\
                     3. Windows: Check 'readonly' field\n\
                     4. If no write permission, use different path or fail gracefully\n\
                     ```\n\n\
                     PATTERN 3: Chunking strategy\n\
                     ```\n\
                     1. Get file with fs_get_file_info\n\
                     2. Divide by 100: chunk_size = line_count / 100\n\
                     3. Use fs_read_file with offset 0, length chunk_size\n\
                     4. Increment offset, repeat for remaining chunks\n\
                     ```\n\n\
                     PATTERN 4: Directory inspection\n\
                     ```\n\
                     1. Call fs_get_file_info({\"path\": \".\"})\n\
                     2. If is_directory is false, path is a file (not a folder)\n\
                     3. For dir contents, use fs_list_directory (not fs_get_file_info)\n\
                     ```\n\n\
                     PATTERN 5: Modified time checks\n\
                     ```\n\
                     1. Get metadata: fs_get_file_info({\"path\": \"cache.json\"})\n\
                     2. If modified_secs_ago > 3600: Cache is stale (>1 hour old)\n\
                     3. Regenerate cache or refresh from source\n\
                     ```",
                ),
            },
        ])
    }
}
