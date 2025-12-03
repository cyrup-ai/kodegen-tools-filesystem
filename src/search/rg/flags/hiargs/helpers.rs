/*!
Helper functions for building high-level arguments.

Contains utility functions for constructing various configuration objects.
*/

use std::path::{Path, PathBuf};

use super::types::State;
use crate::search::rg::flags::lowargs::{LowArgs, TypeChange};

/// Builds the file type matcher from low level arguments.
pub(crate) fn types(low: &LowArgs) -> anyhow::Result<ignore::types::Types> {
    let mut builder = ignore::types::TypesBuilder::new();
    builder.add_defaults();
    for tychange in &low.type_changes {
        match *tychange {
            TypeChange::Select { ref name } => {
                builder.select(name);
            }
            TypeChange::Negate { ref name } => {
                builder.negate(name);
            }
        }
    }
    Ok(builder.build()?)
}

/// Builds a glob matcher for all of the preprocessor globs (via `--pre-glob`).
pub(crate) fn preprocessor_globs(
    state: &State,
    low: &LowArgs,
) -> anyhow::Result<ignore::overrides::Override> {
    if low.pre_glob.is_empty() {
        return Ok(ignore::overrides::Override::empty());
    }
    let mut builder = ignore::overrides::OverrideBuilder::new(&state.cwd);
    for glob in &low.pre_glob {
        builder.add(glob)?;
    }
    Ok(builder.build()?)
}

/// Attempts to discover the current working directory.
///
/// Prefers client's pwd from ToolExecutionContext when available (for HTTP MCP servers).
/// Falls back to server's pwd for non-HTTP usage (library usage, tests).
/// Final fallback to PWD environment variable in exotic circumstances.
pub(crate) fn current_dir(client_pwd: Option<&Path>) -> anyhow::Result<PathBuf> {
    // PRIORITY 1: Use client's pwd if available (HTTP MCP server scenario)
    if let Some(pwd) = client_pwd {
        return Ok(pwd.to_path_buf());
    }

    // PRIORITY 2: Fallback to server's pwd for non-HTTP usage
    let err = match std::env::current_dir() {
        Err(err) => err,
        Ok(cwd) => return Ok(cwd),
    };

    // PRIORITY 3: Final fallback to PWD environment variable
    if let Some(cwd) = std::env::var_os("PWD")
        && !cwd.is_empty()
    {
        return Ok(PathBuf::from(cwd));
    }

    anyhow::bail!(
        "failed to get current working directory: {err}\n\
         did your CWD get deleted?",
    )
}

/// Possibly suggest another regex engine based on the error message given.
///
/// This inspects an error resulting from building a Rust regex matcher, and
/// if it's believed to correspond to a syntax error that another engine could
/// handle, then add a message to suggest the use of the engine flag.
pub(crate) fn suggest_other_engine(msg: String) -> String {
    if let Some(pcre_msg) = suggest_pcre2(&msg) {
        return pcre_msg;
    }
    msg
}

/// Possibly suggest PCRE2 based on the error message given.
///
/// Inspect an error resulting from building a Rust regex matcher, and if it's
/// believed to correspond to a syntax error that PCRE2 could handle, then
/// add a message to suggest the use of -P/--pcre2.
pub(crate) fn suggest_pcre2(msg: &str) -> Option<String> {
    // PCRE2 is always available
    if !msg.contains("backreferences") && !msg.contains("look-around") {
        None
    } else {
        Some(format!(
            "{msg}

Consider enabling PCRE2 with the --pcre2 flag, which can handle backreferences
and look-around.",
        ))
    }
}

/// Possibly suggest multiline mode based on the error message given.
///
/// Uses heuristic pattern matching on the error message, and if it
/// looks like the user tried to type a literal line terminator then it will
/// return a new error message suggesting the use of -U/--multiline.
pub(crate) fn suggest_multiline(msg: String) -> String {
    if msg.contains("the literal") && msg.contains("not allowed") {
        format!(
            "{msg}

Consider enabling multiline mode with the --multiline flag (or -U for short).
When multiline mode is enabled, new line characters can be matched.",
        )
    } else {
        msg
    }
}

/// Possibly suggest the `-a/--text` flag.
pub(crate) fn suggest_text(msg: String) -> String {
    if msg.contains("pattern contains \"\\0\"") {
        format!(
            "{msg}

Consider enabling text mode with the --text flag (or -a for short). Otherwise,
binary detection is enabled and matching a NUL byte is impossible.",
        )
    } else {
        msg
    }
}
