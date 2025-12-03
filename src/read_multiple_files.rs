use crate::ReadFileTool;
use futures::future;
use kodegen_mcp_schema::filesystem::{FileReadResult, FsReadMultipleFilesArgs, FsReadMultipleFilesOutput, FsReadMultipleFilesPromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};

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
    type PromptArgs = FsReadMultipleFilesPromptArgs;

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

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

        let summary = format!(
            "\x1b[36m󰄶 Read multiple files (parallel)\x1b[0m\n 󰗚 Results: {files_read} successful · {files_failed} failed of {files_requested} total"
        );

        Ok(ToolResponse::new(summary, FsReadMultipleFilesOutput {
            success: true,
            files_requested,
            files_read,
            files_failed,
            results,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "file_type".to_string(),
            title: None,
            description: Some(
                "Optional file type to focus examples on (e.g., 'json', 'log', 'rust')"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I read multiple files at once?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The read_multiple_files tool reads multiple files in parallel:\n\n\
                     1. Basic usage:\n\
                        read_multiple_files({\n\
                          \"paths\": [\"/path/file1.txt\", \"/path/file2.json\", \"/path/image.png\"]\n\
                        })\n\n\
                     2. With offset/length:\n\
                        read_multiple_files({\n\
                          \"paths\": [\"file1.txt\", \"file2.txt\"],\n\
                          \"offset\": 0,\n\
                          \"length\": 100\n\
                        })\n\n\
                     3. Read last 30 lines from multiple files:\n\
                        read_multiple_files({\n\
                          \"paths\": [\"log1.txt\", \"log2.txt\"],\n\
                          \"offset\": -30\n\
                        })\n\n\
                     Benefits:\n\
                     - Reads files in parallel for better performance\n\
                     - Returns results for ALL files, even if some fail\n\
                     - Each result includes content OR error\n\
                     - Handles text files, images, and mixed types\n\
                     - Same validation and features as read_file\n\
                     - Supports negative offsets for tail behavior (length ignored)\n\n\
                     Response format:\n\
                     - results: Array of file results\n\
                     - summary: Total, successful, and failed counts\n\n\
                     Use this instead of calling read_file multiple times sequentially.",
                ),
            },
        ])
    }
}
