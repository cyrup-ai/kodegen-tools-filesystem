use crate::{validate_path, display_path_relative_to_git_root};
use kodegen_mcp_schema::filesystem::{FsCreateDirectoryArgs, FsCreateDirectoryPromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use tokio::fs;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct CreateDirectoryTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl CreateDirectoryTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for CreateDirectoryTool {
    type Args = FsCreateDirectoryArgs;
    type PromptArgs = FsCreateDirectoryPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_CREATE_DIRECTORY
    }

    fn description() -> &'static str {
        "Create a new directory or ensure a directory exists. Can create multiple nested \
         directories in one operation. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false // Creates only, doesn't delete
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        fs::create_dir_all(&valid_path).await?;

        let mut contents = Vec::new();

        // Human summary
        let display_path = display_path_relative_to_git_root(&valid_path, ctx.git_root());
        let summary = format!(
            "\x1b[32m󰉋 Created directory: {}\x1b[0m\n\
             󰄴 Status: Directory ready (idempotent)",
            display_path
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "path": valid_path.to_string_lossy(),
            "created": true
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario".to_string(),
            title: None,
            description: Some(
                "Use case for customized examples: 'basic' (single directory), 'nested' (hierarchy), \
                 'idempotence' (repeated calls), or 'validation' (path behavior)"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let scenario = args.scenario.as_deref();

        let content = match scenario {
            Some("basic") => {
                "The create_directory tool creates a single directory.\n\n\
                 Basic usage:\n\
                 create_directory({\"path\": \"/path/to/newdir\"})\n\n\
                 This creates the directory 'newdir' at the specified path. If the parent directories \
                 already exist, only the final directory is created. The tool is idempotent - calling \
                 it multiple times with the same path is safe and will succeed even if the directory \
                 already exists.\n\n\
                 Common use case: Create a directory before writing files into it."
            }
            Some("nested") => {
                "The create_directory tool excels at creating deep directory hierarchies in one operation.\n\n\
                 Nested directory example:\n\
                 create_directory({\"path\": \"/projects/myapp/src/components/ui/buttons\"})\n\n\
                 This single call creates ALL parent directories automatically:\n\
                 - /projects\n\
                 - /projects/myapp\n\
                 - /projects/myapp/src\n\
                 - /projects/myapp/src/components\n\
                 - /projects/myapp/src/components/ui\n\
                 - /projects/myapp/src/components/ui/buttons\n\n\
                 You don't need to create each level separately. The tool uses create_dir_all internally, \
                 which handles the entire hierarchy. This is the recommended approach for setting up \
                 project structures or organizing files in nested categories."
            }
            Some("idempotence") => {
                "The create_directory tool is idempotent - you can safely call it multiple times.\n\n\
                 Example - calling twice with same path:\n\
                 create_directory({\"path\": \"/tmp/cache\"})\n\
                 create_directory({\"path\": \"/tmp/cache\"})  // Also succeeds!\n\n\
                 Both calls succeed. The second call detects the directory already exists and returns \
                 successfully without error. This makes the tool safe to use in:\n\
                 - Initialization scripts that may run multiple times\n\
                 - Retry logic where operations might be repeated\n\
                 - Concurrent workflows where multiple processes might create the same directory\n\n\
                 The tool's behavior is marked as idempotent=true, destructive=false, making it safe \
                 for repeated invocation without side effects."
            }
            Some("validation") => {
                "The create_directory tool validates and normalizes all paths before creation.\n\n\
                 Path normalization examples:\n\
                 create_directory({\"path\": \"~/projects/myapp\"})\n\
                 // Expands ~ to your home directory: /home/user/projects/myapp\n\n\
                 create_directory({\"path\": \"/tmp/./cache/../data\"})\n\
                 // Normalizes to: /tmp/data\n\n\
                 Security validation:\n\
                 The tool checks that paths are within allowed directories (configured in your system). \
                 Attempts to create directories outside allowed locations will be rejected with an error. \
                 This prevents accidental or malicious directory creation in sensitive system areas.\n\n\
                 The validation happens before any filesystem operations, ensuring safe and predictable \
                 behavior."
            }
            _ => {
                // Default: comprehensive overview
                "The create_directory tool creates directories recursively with automatic validation.\n\n\
                 Basic usage:\n\
                 create_directory({\"path\": \"/path/to/newdir\"})\n\n\
                 Nested directories:\n\
                 create_directory({\"path\": \"/path/to/nested/deep/dir\"})\n\n\
                 Key features:\n\
                 • Recursive creation: Automatically creates all parent directories if they don't exist\n\
                 • Idempotent: Safe to call multiple times with the same path - succeeds even if directory exists\n\
                 • Path validation: Validates paths are within allowed directories before creation\n\
                 • Path normalization: Expands ~ for home directories and normalizes path separators\n\
                 • Non-destructive: Creates only, never deletes (destructive=false)\n\n\
                 Common patterns:\n\
                 1. Before file operations: Always ensure target directory exists\n\
                 2. Project setup: Create entire directory structures in one call\n\
                 3. Initialization: Safe to call in scripts that may run multiple times\n\n\
                 The tool uses tokio::fs::create_dir_all internally for non-blocking async I/O."
            }
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I create directories?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(content),
            },
        ])
    }
}
