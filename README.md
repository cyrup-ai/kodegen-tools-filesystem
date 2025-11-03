# kodegen-tools-filesystem

Memory-efficient, blazing-fast MCP (Model Context Protocol) filesystem tools for AI code generation agents.

[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)

## Overview

`kodegen-tools-filesystem` provides a comprehensive suite of 14 filesystem and search tools exposed via the Model Context Protocol (MCP). Built on top of ripgrep's powerful search capabilities, it offers high-performance file operations, directory management, and advanced code search functionality for AI agents.

## Features

### File Operations
- **Read Files**: Single or batch file reading with offset/length support
- **Write Files**: Create or append with intelligent chunking
- **Edit Files**: Surgical text replacement with exact string matching
- **Move/Delete**: Rename, move, and delete file operations
- **File Info**: Retrieve comprehensive file metadata

### Directory Management
- **List Directories**: Recursive listing with configurable depth
- **Create Directories**: Recursive directory creation
- **Delete Directories**: Safe recursive removal

### Advanced Search (Powered by ripgrep)
- **File Search**: Find files by name pattern with glob support
- **Content Search**: Full-text search inside files with regex/PCRE2
- **Streaming Results**: Asynchronous search with pagination
- **Session Management**: Concurrent searches with cancellation support

### Search Features
- Dual regex engines (Rust regex + PCRE2 fallback)
- Case-sensitive, case-insensitive, and smart-case modes
- Word and line boundary matching
- Binary file handling (auto-detect, skip, or force text)
- Multiline pattern support
- Search inside compressed files (.gz, .zip, .bz2, .xz)
- Sort by path, modification time, access time, or creation time
- Context lines (before/after match)
- Invert match (show non-matching results)

## Installation

### Prerequisites
- Rust nightly toolchain (edition 2024)
- Cargo

### From Source

```bash
git clone https://github.com/cyrup-ai/kodegen-tools-filesystem
cd kodegen-tools-filesystem
cargo build --release
```

The compiled binary will be available at `target/release/kodegen-filesystem`.

## Usage

### Running the Server

```bash
# Run with default settings
cargo run --bin kodegen-filesystem

# Run with custom allowed directories
KODEGEN_ALLOWED_DIRS="/path/to/workspace:/another/path" cargo run --bin kodegen-filesystem

# Run the release build
./target/release/kodegen-filesystem
```

### Configuration

The server respects the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `KODEGEN_ALLOWED_DIRS` | Colon-separated list of allowed directories | Empty (all paths allowed) |
| `KODEGEN_DENIED_DIRS` | Colon-separated list of denied directories | Empty (no paths denied) |

**Path Access Rules:**
1. Denied directories are checked first (blacklist takes precedence)
2. If `KODEGEN_ALLOWED_DIRS` is set, only those paths are accessible
3. If both are empty, all filesystem paths are accessible

### Available Tools

The server exposes 14 MCP tools:

| Category | Tool | Description |
|----------|------|-------------|
| File Ops | `read_file` | Read file contents with offset/length support |
| | `read_multiple_files` | Batch read multiple files |
| | `write_file` | Write or append to files |
| | `edit_block` | Replace text blocks surgically |
| | `move_file` | Move or rename files |
| | `delete_file` | Delete files |
| | `get_file_info` | Get file metadata |
| Directory | `create_directory` | Create directories recursively |
| | `list_directory` | List directory contents with depth |
| | `delete_directory` | Delete directories recursively |
| Search | `start_search` | Start file or content search |
| | `get_more_search_results` | Paginate search results |
| | `stop_search` | Cancel active search |
| | `list_searches` | List active search sessions |

## Examples

### Running Examples

```bash
# Comprehensive demo of all 14 tools
cargo run --example filesystem_demo

# Search examples
cargo run --example direct_search_basics
cargo run --example direct_search_patterns
cargo run --example direct_search_files
cargo run --example direct_search_output
cargo run --example direct_search_advanced
```

### Search Example

```rust
use kodegen_mcp_client::tools;
use serde_json::json;

// Start a content search
let response = client.call_tool(
    tools::START_SEARCH,
    json!({
        "path": "/path/to/search",
        "pattern": "TODO",
        "search_type": "content",
        "case_mode": "insensitive",
        "max_results": 100
    })
).await?;

let session_id = response.session_id;

// Get results
let results = client.call_tool(
    tools::GET_MORE_SEARCH_RESULTS,
    json!({
        "session_id": session_id,
        "offset": 0,
        "length": 50
    })
).await?;
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Auto-fix clippy warnings
cargo clippy --fix
```

## Architecture

### Core Components

- **Tool Modules**: Each tool (`read_file`, `write_file`, etc.) is a self-contained module
- **Search Manager**: Stateful coordinator for concurrent search sessions
- **Ripgrep Integration**: Full ripgrep implementation with dual regex engines
- **Path Validation**: Security layer for filesystem access control
- **HTTP Server**: MCP protocol server using `kodegen_server_http`

### Search Architecture

The search system uses a session-based model:
1. `start_search` creates a background search task and returns a session ID
2. `get_more_search_results` retrieves paginated results
3. `stop_search` cancels the background task
4. Sessions auto-expire after inactivity

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE.md) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE.md) or http://opensource.org/licenses/MIT)

at your option.

## Links

- Homepage: [kodegen.ai](https://kodegen.ai)
- Repository: [github.com/cyrup-ai/kodegen-tools-filesystem](https://github.com/cyrup-ai/kodegen-tools-filesystem)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

**Built with ❤️ by KODEGEN.ᴀɪ**
