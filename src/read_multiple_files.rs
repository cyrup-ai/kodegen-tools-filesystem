use crate::ReadFileTool;
use futures::future;
use kodegen_mcp_schema::filesystem::{FsReadMultipleFilesArgs, FsReadMultipleFilesPromptArgs};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

// ============================================================================
// HELPER TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MultiFileResult {
    /// Path to the file
    pub path: String,

    /// File content (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// MIME type (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Whether file is an image (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_image: Option<bool>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

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

    /// Read a single file and convert to `MultiFileResult`
    async fn read_one_file(
        &self,
        path: String,
        offset: i64,
        length: Option<usize>,
    ) -> MultiFileResult {
        use kodegen_mcp_schema::filesystem::FsReadFileArgs;

        let args = FsReadFileArgs {
            path: path.clone(),
            offset,
            length,
            is_url: false,
        };

        match self.read_file_tool.execute(args).await {
            Ok(contents) => {
                // Parse the JSON metadata (second Content item)
                if contents.len() >= 2
                    && let Some(json_content) = contents.get(1)
                    && let rmcp::model::RawContent::Text(text_content) = &json_content.raw
                    && let Ok(result) = serde_json::from_str::<Value>(&text_content.text)
                {
                    return MultiFileResult {
                        path,
                        content: result
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .or_else(|| {
                                contents.first().and_then(|c| {
                                    if let rmcp::model::RawContent::Text(text_content) = &c.raw {
                                        Some(text_content.text.clone())
                                    } else {
                                        None
                                    }
                                })
                            }),
                        mime_type: result
                            .get("mime_type")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        is_image: result.get("is_image").and_then(serde_json::Value::as_bool),
                        error: None,
                    };
                }
                
                // Fallback if parsing fails
                MultiFileResult {
                    path,
                    content: contents.first().and_then(|c| {
                        if let rmcp::model::RawContent::Text(text_content) = &c.raw {
                            Some(text_content.text.clone())
                        } else {
                            None
                        }
                    }),
                    mime_type: None,
                    is_image: None,
                    error: None,
                }
            }
            Err(e) => MultiFileResult {
                path,
                content: None,
                mime_type: None,
                is_image: None,
                error: Some(e.to_string()),
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
        "fs_read_multiple_files"
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        if args.paths.is_empty() {
            return Err(McpError::InvalidArguments(
                "No paths provided. Please provide at least one file path.".to_string(),
            ));
        }

        // Create futures for all file reads
        let read_futures = args
            .paths
            .into_iter()
            .map(|path| self.read_one_file(path, args.offset, args.length));

        // Execute all reads in parallel
        let results = future::join_all(read_futures).await;

        // Count successes and failures
        let total = results.len();
        let successful = results.iter().filter(|r| r.error.is_none()).count();
        let failed = total - successful;

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let mut summary_lines = vec![format!(
            "📚 Read {} files ({} successful, {} failed)\n",
            total, successful, failed
        )];

        for result in &results {
            if result.error.is_none() {
                let lines_info = if let Some(content) = &result.content {
                    let line_count = content.lines().count();
                    format!(" ({} lines)", line_count)
                } else {
                    String::new()
                };
                summary_lines.push(format!("✓ {}{}", result.path, lines_info));
            } else {
                let error_msg = result.error.as_ref().unwrap();
                summary_lines.push(format!("✗ {} ({})", result.path, error_msg));
            }
        }

        let summary = summary_lines.join("\n");
        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "total_files": total,
            "successful": successful,
            "failed": failed,
            "results": results
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
