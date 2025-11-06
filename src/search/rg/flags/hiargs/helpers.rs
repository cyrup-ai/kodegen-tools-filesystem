/*!
Helper functions for building high-level arguments.

Contains utility functions for constructing various configuration objects.
*/

use std::path::{Path, PathBuf};

use grep::printer::ColorSpecs;

use super::types::State;
use crate::search::rg::flags::lowargs::{LowArgs, Mode, SearchMode, TypeChange};

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

/// Builds the glob "override" matcher from the CLI `-g/--glob` and `--iglob`
/// flags.
pub(crate) fn globs(state: &State, low: &LowArgs) -> anyhow::Result<ignore::overrides::Override> {
    if low.globs.is_empty() && low.iglobs.is_empty() {
        return Ok(ignore::overrides::Override::empty());
    }
    let mut builder = ignore::overrides::OverrideBuilder::new(&state.cwd);
    // Make all globs case insensitive with --glob-case-insensitive.
    if low.glob_case_insensitive {
        builder.case_insensitive(true)?;
    }
    for glob in &low.globs {
        builder.add(glob)?;
    }
    // This only enables case insensitivity for subsequent globs.
    builder.case_insensitive(true)?;
    for glob in &low.iglobs {
        builder.add(glob)?;
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

/// Determines whether stats should be tracked for this search. If so, a stats
/// object is returned.
pub(crate) fn stats(low: &LowArgs) -> Option<grep::printer::Stats> {
    if !matches!(low.mode, Mode::Search(_)) {
        return None;
    }
    if low.stats || matches!(low.mode, Mode::Search(SearchMode::Json)) {
        return Some(grep::printer::Stats::new());
    }
    None
}

/// Pulls out any color specs provided by the user and assembles them into one
/// single configuration.
pub(crate) fn take_color_specs(_: &mut State, low: &mut LowArgs) -> ColorSpecs {
    let mut specs = grep::printer::default_color_specs();
    for spec in low.colors.drain(..) {
        specs.push(spec);
    }
    ColorSpecs::new(&specs)
}

/// Pulls out the necessary info from the low arguments to build a full
/// hyperlink configuration.
pub(crate) fn take_hyperlink_config(
    _: &mut State,
    low: &mut LowArgs,
) -> anyhow::Result<grep::printer::HyperlinkConfig> {
    let mut env = grep::printer::HyperlinkEnvironment::new();
    if let Some(hostname) = hostname(low.hostname_bin.as_deref()) {
        log::debug!("found hostname for hyperlink configuration: {hostname}");
        env.host(Some(hostname));
    }
    if let Some(wsl_prefix) = wsl_prefix() {
        log::debug!("found wsl_prefix for hyperlink configuration: {wsl_prefix}");
        env.wsl_prefix(Some(wsl_prefix));
    }
    let fmt = std::mem::take(&mut low.hyperlink_format);
    log::debug!("hyperlink format: {:?}", fmt.to_string());
    Ok(grep::printer::HyperlinkConfig::new(env, fmt))
}

/// Attempts to discover the current working directory.
///
/// This mostly just defers to the standard library, however, such things will
/// fail if ripgrep is in a directory that no longer exists. We attempt some
/// fallback mechanisms, such as querying the PWD environment variable, but
/// otherwise return an error.
pub(crate) fn current_dir() -> anyhow::Result<PathBuf> {
    let err = match std::env::current_dir() {
        Err(err) => err,
        Ok(cwd) => return Ok(cwd),
    };
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

/// Retrieves the hostname that should be used wherever a hostname is required.
///
/// Currently, this is only used in the hyperlink format.
///
/// This works by first running the given binary program (if present and with
/// no arguments) to get the hostname after trimming leading and trailing
/// whitespace. If that fails for any reason, then it falls back to getting
/// the hostname via platform specific means (e.g., `gethostname` on Unix).
///
/// The purpose of `bin` is to make it possible for end users to override how
/// ripgrep determines the hostname.
pub(crate) fn hostname(bin: Option<&Path>) -> Option<String> {
    let Some(bin) = bin else {
        return platform_hostname();
    };
    let bin = match grep::cli::resolve_binary(bin) {
        Ok(bin) => bin,
        Err(err) => {
            log::debug!(
                "failed to run command '{bin:?}' to get hostname \
                 (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let mut cmd = std::process::Command::new(&bin);
    cmd.stdin(std::process::Stdio::null());
    let rdr = match grep::cli::CommandReader::new(&mut cmd) {
        Ok(rdr) => rdr,
        Err(err) => {
            log::debug!(
                "failed to spawn command '{bin:?}' to get \
                 hostname (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let out = match std::io::read_to_string(rdr) {
        Ok(out) => out,
        Err(err) => {
            log::debug!(
                "failed to read output from command '{bin:?}' to get \
                 hostname (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let hostname = out.trim();
    if hostname.is_empty() {
        log::debug!(
            "output from command '{bin:?}' is empty after trimming \
             leading and trailing whitespace (falling back to \
             platform hostname)",
        );
        return platform_hostname();
    }
    Some(hostname.to_string())
}

/// Attempts to get the hostname by using platform specific routines.
///
/// For example, this will do `gethostname` on Unix and `GetComputerNameExW` on
/// Windows.
pub(crate) fn platform_hostname() -> Option<String> {
    let hostname_os = match grep::cli::hostname() {
        Ok(x) => x,
        Err(err) => {
            log::debug!("could not get hostname: {err}");
            return None;
        }
    };
    let Some(hostname) = hostname_os.to_str() else {
        log::debug!("got hostname {hostname_os:?}, but it's not valid UTF-8");
        return None;
    };
    Some(hostname.to_string())
}

/// Returns the value for the `{wslprefix}` variable in a hyperlink format.
///
/// A WSL prefix is a share/network like thing that is meant to permit Windows
/// applications to open files stored within a WSL drive.
///
/// If a WSL distro name is unavailable, not valid UTF-8 or this isn't running
/// in a Unix environment, then this returns None.
///
/// See: <https://learn.microsoft.com/en-us/windows/wsl/filesystems>
pub(crate) fn wsl_prefix() -> Option<String> {
    if !cfg!(unix) {
        return None;
    }
    let distro_os = std::env::var_os("WSL_DISTRO_NAME")?;
    let Some(distro) = distro_os.to_str() else {
        log::debug!("found WSL_DISTRO_NAME={distro_os:?}, but value is not UTF-8");
        return None;
    };
    Some(format!("wsl$/{distro}"))
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
