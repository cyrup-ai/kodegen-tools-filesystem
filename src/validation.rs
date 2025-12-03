use kodegen_mcp_tool::error::McpError;
use log::warn;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::time::{Duration, timeout};

/// Normalize all paths consistently
/// On Windows: lowercase for case-insensitive matching
/// On Unix-like systems: preserve case for case-sensitive matching
fn normalize_path(p: &str) -> String {
    let expanded = expand_home(p);
    
    #[cfg(windows)]
    {
        expanded.to_lowercase()
    }
    
    #[cfg(not(windows))]
    {
        expanded
    }
}

/// Expand home directory (~) in file paths
fn expand_home(filepath: &str) -> String {
    if (filepath.starts_with("~/") || filepath == "~")
        && let Some(home_dir) = dirs::home_dir()
    {
        return home_dir.join(&filepath[1..]).to_string_lossy().to_string();
    }
    filepath.to_string()
}

/// Get the list of allowed directories from config
fn get_allowed_dirs(config: &kodegen_config_manager::ServerConfig) -> &[String] {
    &config.allowed_directories
}

/// Get the list of denied directories from config
fn get_denied_dirs(config: &kodegen_config_manager::ServerConfig) -> &[String] {
    &config.denied_directories
}

/// Returns (`is_allowed`, `restriction_reason`)
/// `restriction_reason` is `Some(message)` if denied, `None` if allowed
fn is_path_allowed(
    path_to_check: &str,
    config: &kodegen_config_manager::ServerConfig,
) -> (bool, Option<String>) {
    let allowed_dirs = get_allowed_dirs(config);
    let denied_dirs = get_denied_dirs(config);

    let mut normalized_path_to_check = normalize_path(path_to_check);
    if normalized_path_to_check.ends_with(std::path::MAIN_SEPARATOR) {
        normalized_path_to_check.pop();
    }

    // STEP 1: Check denied list first (blacklist takes precedence)
    if !denied_dirs.is_empty() {
        let denied_match = denied_dirs.iter().find(|denied_dir| {
            let mut normalized_denied = normalize_path(denied_dir);
            if normalized_denied.ends_with(std::path::MAIN_SEPARATOR) {
                normalized_denied.pop();
            }

            // Exact match or subdirectory
            normalized_path_to_check == normalized_denied
                || normalized_path_to_check.starts_with(&format!(
                    "{}{}",
                    normalized_denied,
                    std::path::MAIN_SEPARATOR
                ))
        });

        if let Some(denied_dir) = denied_match {
            let reason = format!(
                "Path is in denied directory: {denied_dir}\n\
                 Current denied directories: {denied_dirs:?}\n\
                 \n\
                 To modify restrictions:\n\
                 1. Remove from blacklist: unset KODEGEN_DENIED_DIRS or use set_config_value tool\n\
                 2. Or add to whitelist: export KODEGEN_ALLOWED_DIRS=\"{path_to_check}:$KODEGEN_ALLOWED_DIRS\""
            );
            return (false, Some(reason));
        }
    }

    // STEP 2: Check whitelist (if non-empty)
    if !allowed_dirs.is_empty() {
        // If root directory is allowed, all paths are allowed
        if allowed_dirs.contains(&"/".to_string()) {
            return (true, None);
        }

        let is_allowed = allowed_dirs.iter().any(|allowed_dir| {
            let mut normalized_allowed = normalize_path(allowed_dir);
            if normalized_allowed.ends_with(std::path::MAIN_SEPARATOR) {
                normalized_allowed.pop();
            }

            // Exact match
            if normalized_path_to_check == normalized_allowed {
                return true;
            }

            // Subdirectory check
            let subdir_check = normalized_path_to_check.starts_with(&format!(
                "{}{}",
                normalized_allowed,
                std::path::MAIN_SEPARATOR
            ));

            // Windows drive check
            if cfg!(windows) && normalized_allowed.ends_with(':') {
                return normalized_path_to_check.starts_with(&normalized_allowed);
            }

            subdir_check
        });

        if !is_allowed {
            let reason = format!(
                "Path not in allowed directories\n\
                 Current allowed directories: {allowed_dirs:?}\n\
                 \n\
                 To allow access to this path:\n\
                 1. Via environment variable: export KODEGEN_ALLOWED_DIRS=\"{path_to_check}:$KODEGEN_ALLOWED_DIRS\"\n\
                 2. Via MCP tool: set_config_value({{\"key\": \"allowed_directories\", \"value\": [\"{path_to_check}\", ...]}})\n\
                 3. Or allow all: export KODEGEN_ALLOWED_DIRS=\"/\""
            );
            return (false, Some(reason));
        }

        return (true, None);
    }

    // STEP 3: No restrictions = allow all
    (true, None)
}

/// Validates a path to ensure it can be accessed or created
///
/// # Errors
/// Returns error if path is denied, validation times out, or parent directories are invalid
pub async fn validate_path(
    requested_path: &str,
    config_manager: &kodegen_config_manager::ConfigManager,
    client_pwd: Option<&Path>,
) -> Result<PathBuf, McpError> {
    // Get timeout from configuration (default: 30 seconds)
    let timeout_ms = config_manager.get_path_validation_timeout_ms();

    let validation_operation = async {
        // Get current config
        let config = config_manager.get_config();

        // Expand home directory if present
        let expanded_path = expand_home(requested_path);

        // Convert to absolute path
        let absolute = if Path::new(&expanded_path).is_absolute() {
            PathBuf::from(&expanded_path)
        } else {
            // Use client's pwd if available, fallback to server's pwd
            let base_dir = if let Some(pwd) = client_pwd {
                pwd.to_path_buf()
            } else {
                // Fallback for non-HTTP clients (direct library usage, tests)
                std::env::current_dir().map_err(McpError::Io)?
            };
            base_dir.join(&expanded_path)
        };

        // Check if path is allowed (get detailed error)
        let (is_allowed, restriction_reason) =
            is_path_allowed(&absolute.to_string_lossy(), &config);
        if !is_allowed {
            let error_msg =
                restriction_reason.unwrap_or_else(|| format!("Path not allowed: {requested_path}"));
            warn!("Path access denied: {requested_path}");
            return Err(McpError::PermissionDenied(error_msg));
        }

        // Check if path exists
        match fs::metadata(&absolute).await {
            Ok(_) => {
                // If path exists, resolve any symlinks
                match fs::canonicalize(&absolute).await {
                    Ok(canonical) => Ok(canonical),
                    Err(_) => Ok(absolute), // Fall back to absolute path
                }
            }
            Err(_) => {
                // Path doesn't exist, return absolute for operations that create paths
                Ok(absolute)
            }
        }
    };

    // Execute with configurable timeout
    if let Ok(result) = timeout(
        Duration::from_millis(timeout_ms),
        validation_operation,
    )
    .await
    {
        result
    } else {
        warn!(
            "Path validation timeout for {} ({}ms)",
            requested_path, timeout_ms
        );
        Err(McpError::Other(anyhow::anyhow!(
            "Path validation timeout after {}ms. \
             For slow network filesystems, increase timeout via: \
             set_config_value({{\"key\": \"path_validation_timeout_ms\", \"value\": 60000}})",
            timeout_ms
        )))
    }
}

/// Convert an absolute path to a display string relative to git_root if available
///
/// This function formats paths for user-facing output (Content[0]):
/// - If git_root is available and path is within it: returns relative path
/// - Otherwise: returns absolute path as-is
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use kodegen_tools_filesystem::display_path_relative_to_git_root;
///
/// let git_root = Some(Path::new("/Users/davidmaple/kodegen-workspace"));
/// let file_path = Path::new("/Users/davidmaple/kodegen-workspace/packages/kodegen-utils/Cargo.toml");
/// let display = display_path_relative_to_git_root(file_path, git_root);
/// assert_eq!(display, "packages/kodegen-utils/Cargo.toml");
/// ```
///
/// ```
/// use std::path::Path;
/// use kodegen_tools_filesystem::display_path_relative_to_git_root;
///
/// // When git_root is not available, return absolute path
/// let file_path = Path::new("/Users/davidmaple/kodegen-workspace/packages/kodegen-utils/Cargo.toml");
/// let display = display_path_relative_to_git_root(file_path, None);
/// assert_eq!(display, "/Users/davidmaple/kodegen-workspace/packages/kodegen-utils/Cargo.toml");
/// ```
pub fn display_path_relative_to_git_root(
    path: &Path,
    git_root: Option<&Path>,
) -> String {
    if let Some(root) = git_root {
        // Try to strip the git root prefix
        if let Ok(relative) = path.strip_prefix(root) {
            // Return relative path as string
            return relative.display().to_string();
        }
    }
    
    // Fallback: return absolute path if git_root unavailable or path not within git_root
    path.display().to_string()
}
