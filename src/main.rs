//! Filesystem Category HTTP Server
//!
//! Serves filesystem tools via HTTP/HTTPS transport using kodegen_server_http.

use anyhow::Result;
use kodegen_server_http::{run_http_server, Managers, RouterSet, register_tool, ConnectionCleanupFn};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
use std::future::Future;
use std::pin::Pin;

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
            kodegen_tools_filesystem::search::FsSearchTool::new(search_registry.clone()),
        );

        // Create cleanup callback for connection dropped notification
        let cleanup: ConnectionCleanupFn = std::sync::Arc::new(move |connection_id: String| {
            let registry = search_registry.clone();
            Box::pin(async move {
                let cleaned = registry.cleanup_connection(&connection_id).await;
                log::info!(
                    "Connection {}: cleaned up {} search session(s)",
                    connection_id,
                    cleaned
                );
            }) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        });

        let mut router_set = RouterSet::new(tool_router, prompt_router, managers);
        router_set.connection_cleanup = Some(cleanup);
        Ok(router_set)
        })
    })
    .await
}
