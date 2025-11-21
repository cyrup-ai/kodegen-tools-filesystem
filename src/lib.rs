mod validation;
pub use validation::*;

pub mod read_file;
pub use read_file::*;

pub mod read_multiple_files;
pub use read_multiple_files::*;

pub mod write_file;
pub use write_file::*;

pub mod edit_block;
pub use edit_block::*;

pub mod create_directory;
pub use create_directory::*;

pub mod list_directory;
pub use list_directory::*;

pub mod move_file;
pub use move_file::*;

pub mod delete_file;
pub use delete_file::*;

pub mod delete_directory;
pub use delete_directory::*;

pub mod get_file_info;
pub use get_file_info::*;

pub mod search;

/// Start the filesystem HTTP server programmatically
///
/// Returns a ServerHandle for graceful shutdown control.
/// This function is non-blocking - the server runs in background tasks.
///
/// # Arguments
/// * `addr` - Socket address to bind to (e.g., "127.0.0.1:30437")
/// * `tls_cert` - Optional path to TLS certificate file
/// * `tls_key` - Optional path to TLS private key file
///
/// # Returns
/// ServerHandle for graceful shutdown, or error if startup fails
pub async fn start_server(
    addr: std::net::SocketAddr,
    tls_cert: Option<std::path::PathBuf>,
    tls_key: Option<std::path::PathBuf>,
) -> anyhow::Result<kodegen_server_http::ServerHandle> {
    use kodegen_server_http::{create_http_server, Managers, RouterSet, register_tool};
    use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
    use std::time::Duration;

    let tls_config = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => Some((cert, key)),
        _ => None,
    };

    let shutdown_timeout = Duration::from_secs(30);
    let session_keep_alive = Duration::ZERO;

    create_http_server("filesystem", addr, tls_config, shutdown_timeout, session_keep_alive, |config: &kodegen_config_manager::ConfigManager, _tracker| {
        let config = config.clone();
        Box::pin(async move {
            let tool_router = ToolRouter::new();
            let prompt_router = PromptRouter::new();
            let managers = Managers::new();

            let file_read_line_limit = config.get_file_read_line_limit();

            // Register all 14 filesystem tools
            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::ReadFileTool::new(file_read_line_limit, config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::ReadMultipleFilesTool::new(file_read_line_limit, config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::WriteFileTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::MoveFileTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::DeleteFileTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::DeleteDirectoryTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::ListDirectoryTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::CreateDirectoryTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::GetFileInfoTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::EditBlockTool::new(config.clone()),
            );

            // Search tools
            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                crate::search::FsSearchTool::new(),
            );

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    }).await
}
