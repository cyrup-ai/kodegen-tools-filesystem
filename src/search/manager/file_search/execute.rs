//! Execute file search using walker with parallel directory traversal

use super::builder::FileSearchBuilder;
use super::pattern;
use super::visitor::CompiledPattern;
use crate::search::manager::config::DEFAULT_MAX_RESULTS;
use crate::search::types::{BoundaryMode, PatternMode};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Execute file search using walker with parallel directory traversal
pub fn execute(
    options: &crate::search::types::SearchSessionOptions,
    root: &std::path::PathBuf,
    ctx: &mut crate::search::manager::context::SearchContext,
) {
    use crate::search::rg::flags::hiargs::HiArgs;
    use crate::search::rg::flags::lowargs::{LowArgs, Mode, PatternSource, SearchMode};
    use crate::search::manager::utils::{build_type_changes, configure_walker, convert_case_mode};
    use ignore::WalkBuilder;
    use std::sync::atomic::Ordering;

    // Detect pattern type using intelligent inference
    // Priority: user override > literal_search > regex detection > glob detection > substring
    let detected_pattern_type = pattern::detect(
        &options.pattern,
        options.literal_search,
        options.pattern_mode,
    );

    // Store pattern type in context for output
    ctx.pattern_type = Some(detected_pattern_type);

    // Compile pattern based on detected type
    let compiled_pattern = match detected_pattern_type {
        PatternMode::Regex => {
            match regex::Regex::new(&options.pattern) {
                Ok(re) => CompiledPattern::Regex(re),
                Err(e) => {
                    log::warn!("Failed to compile regex '{}': {}, falling back to substring", options.pattern, e);
                    ctx.pattern_type = Some(PatternMode::Substring);
                    CompiledPattern::Substring
                }
            }
        }
        PatternMode::Glob => {
            match globset::Glob::new(&options.pattern) {
                Ok(g) => CompiledPattern::Glob(g.compile_matcher()),
                Err(e) => {
                    log::warn!("Failed to compile glob '{}': {}, falling back to substring", options.pattern, e);
                    ctx.pattern_type = Some(PatternMode::Substring);
                    CompiledPattern::Substring
                }
            }
        }
        PatternMode::Substring => CompiledPattern::Substring,
    };
    let pattern_lower = options.pattern.to_lowercase();

    // Precompute smart case flag once (performance optimization)
    let is_pattern_lowercase = options.pattern.chars().all(|c| !c.is_uppercase());

    let max_results = options.max_results.unwrap_or(DEFAULT_MAX_RESULTS as u32) as usize;

    // Build type_changes for ripgrep
    let type_changes = build_type_changes(options);

    // Build LowArgs for type filtering
    log::debug!(
        "file_search::execute: options.no_ignore = {}",
        options.no_ignore
    );
    let low_args = LowArgs {
        patterns: vec![PatternSource::Regexp(options.pattern.clone())],
        case: convert_case_mode(options.case_mode),
        fixed_strings: options.literal_search,
        hidden: options.include_hidden,
        invert_match: options.invert_match,
        mode: Mode::Search(SearchMode::Standard),
        type_changes,
        // Match ripgrep's --no-ignore flag behavior exactly
        no_ignore_vcs: options.no_ignore,
        no_ignore_exclude: options.no_ignore,
        no_ignore_global: options.no_ignore,
        no_ignore_parent: options.no_ignore,
        no_ignore_dot: options.no_ignore,
        ..Default::default()
    };
    log::debug!(
        "file_search::execute: LowArgs no_ignore_vcs={}, no_ignore_dot={}, no_ignore_parent={}",
        low_args.no_ignore_vcs,
        low_args.no_ignore_dot,
        low_args.no_ignore_parent
    );

    // Build HiArgs for types
    // Pass client_pwd from SearchContext for correct working directory resolution
    let hi_args = match HiArgs::from_low_args(low_args, ctx.client_pwd.as_deref()) {
        Ok(h) => Arc::new(h),
        Err(e) => {
            log::error!("Failed to build HiArgs: {e}");
            ctx.is_complete = true;
            return;
        }
    };
    log::debug!(
        "file_search::execute: HiArgs no_ignore_vcs={}, no_ignore_dot={}, no_ignore_parent={}",
        hi_args.no_ignore_vcs,
        hi_args.no_ignore_dot,
        hi_args.no_ignore_parent
    );

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - handles built-in types + file_pattern
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = FileSearchBuilder {
        compiled_pattern,
        pattern: options.pattern.clone(),
        pattern_lower,
        case_mode: options.case_mode,
        is_pattern_lowercase,
        word_boundary: matches!(options.boundary_mode, Some(BoundaryMode::Word)),
        max_results,
        early_termination: options.early_termination.unwrap_or(false),
        early_term_triggered: Arc::new(AtomicBool::new(false)),
        results: Arc::clone(&ctx.results),
        total_matches: Arc::clone(&ctx.total_matches),
        total_files: Arc::clone(&ctx.total_files),
        error_count: Arc::clone(&ctx.error_count),
        errors: Arc::clone(&ctx.errors),
    };

    // Execute parallel search
    walker.build_parallel().visit(&mut builder);

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "File search completed with {} errors. Pattern: '{}', Path: {}",
            error_count,
            options.pattern,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete = true;
}
