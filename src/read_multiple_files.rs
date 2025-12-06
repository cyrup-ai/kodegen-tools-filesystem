use crate::ReadFileTool;
use futures::future;
use kodegen_config::shorten_path_for_display;
use kodegen_mcp_schema::filesystem::{FileReadResult, FsReadMultipleFilesArgs, FsReadMultipleFilesOutput, ReadMultipleFilesPrompts};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ReadMultipleFilesTool {
    read_file_tool: ReadFileTool,
}

impl ReadMultipleFilesTool {
    #[must_use]
    pub fn new(
        default_line_limit: usize,
        config_manager: kodegen_config_manager::ConfigManager,
    ) -> Self {
        Self {
            read_file_tool: ReadFileTool::new(default_line_limit, config_manager),
        }
    }

    /// Read a single file and convert to `FileReadResult`
    async fn read_one_file(
        &self,
        path: String,
        offset: i64,
        length: Option<usize>,
        ctx: &ToolExecutionContext,
    ) -> FileReadResult {
        use kodegen_mcp_schema::filesystem::FsReadFileArgs;

        let args = FsReadFileArgs {
            path: path.clone(),
            offset,
            length,
            is_url: false,
        };

        match self.read_file_tool.execute(args, ctx.clone()).await {
            Ok(response) => {
                // Extract from the typed output
                FileReadResult {
                    path,
                    success: true,
                    content: Some(response.metadata.content),
                    error: None,
                    mime_type: Some(response.metadata.mime_type),
                }
            }
            Err(e) => FileReadResult {
                path,
                success: false,
                content: None,
                error: Some(e.to_string()),
                mime_type: None,
            },
        }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ReadMultipleFilesTool {
    type Args = FsReadMultipleFilesArgs;
    type Prompts = ReadMultipleFilesPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_READ_MULTIPLE_FILES
    }

    fn description() -> &'static str {
        "Read multiple files in parallel. Returns results for all files, including errors for \
         individual files that fail. Supports offset and length parameters applied to all files. \
         Supports negative offsets for tail behavior (offset: -N reads last N lines). \
         When offset is negative, length is ignored. Automatically validates paths and handles different file types (text/images)."
    }

    fn read_only() -> bool {
        true
    }

    fn open_world() -> bool {
        false // Only reads local files, not URLs
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        if args.paths.is_empty() {
            return Err(McpError::InvalidArguments(
                "No paths provided. Please provide at least one file path.".to_string(),
            ));
        }

        let files_requested = args.paths.len();

        // Create futures for all file reads
        let read_futures = args
            .paths
            .into_iter()
            .map(|path| self.read_one_file(path, args.offset, args.length, &ctx));

        // Execute all reads in parallel
        let results: Vec<FileReadResult> = future::join_all(read_futures).await;

        // Count successes and failures
        let files_read = results.iter().filter(|r| r.success).count();
        let files_failed = files_requested - files_read;

        // Build header line (existing format for backward compatibility)
        let mut summary = format!(
            "\x1b[36m󰄶 Read multiple files (parallel)\x1b[0m\n 󰗚 Results: {files_read} successful · {files_failed} failed of {files_requested} total\n\n\n"
        );

        // Sort results: failures first, then successes (both alphabetically by path)
        let mut sorted_results = results.clone();
        sorted_results.sort_by(|a, b| {
            match (a.success, b.success) {
                // Both success or both failure: sort alphabetically by path
                (true, true) | (false, false) => a.path.cmp(&b.path),
                // Failures come before successes
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
            }
        });

        // Format each file as a single line with icon and shortened path
        for result in &sorted_results {
            let display_path = shorten_path_for_display(
                std::path::Path::new(&result.path),
                ctx.git_root()
            );
            
            if result.success {
                summary.push_str(&format!("\x1b[32m 󰗚 {}\x1b[0m\n", display_path));
            } else {
                summary.push_str(&format!("\x1b[31m 󰅙 {}\x1b[0m\n", display_path));
            }
        }

        Ok(ToolResponse::new(summary, FsReadMultipleFilesOutput {
            success: true,
            files_requested,
            files_read,
            files_failed,
            results,
        }))
    }
}
