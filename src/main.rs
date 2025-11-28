//! Filesystem Category HTTP Server
//!
//! Serves filesystem tools via HTTP/HTTPS transport using kodegen_server_http.

use anyhow::Result;
use kodegen_server_http::{run_http_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_http_server("filesystem", |config, _tracker| {
        let config = config.clone();
        Box::pin(async move {
        let tool_router = ToolRouter::new();
        let prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Get configuration values
        let file_read_line_limit = config.get_file_read_line_limit();

        // Register all 11 filesystem tools
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ReadFileTool::new(
                file_read_line_limit,
                config.clone(),
            ),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ReadMultipleFilesTool::new(
                file_read_line_limit,
                config.clone(),
            ),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::WriteFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::MoveFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::DeleteFileTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::DeleteDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::ListDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::CreateDirectoryTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::GetFileInfoTool::new(config.clone()),
        );

        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::EditBlockTool::new(config.clone()),
        );

        // Search tools - create registry for connection isolation
        let search_registry = std::sync::Arc::new(kodegen_tools_filesystem::search::SearchRegistry::new());
        
        let (tool_router, prompt_router) = register_tool(
            tool_router,
            prompt_router,
            kodegen_tools_filesystem::search::FsSearchTool::new(search_registry),
        );

        Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    })
    .await
}
