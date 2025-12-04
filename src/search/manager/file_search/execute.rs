//! Execute file search using walker with parallel directory traversal

use super::builder::FileSearchBuilder;
use crate::search::manager::config::DEFAULT_MAX_RESULTS;
use crate::search::types::BoundaryMode;
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

    // Only use glob matching if pattern contains actual glob metacharacters
    // Otherwise, use substring matching for intuitive filename search behavior
    let glob_pattern = if options.literal_search {
        None
    } else {
        // Check for glob metacharacters: *, ?, [, {
        let has_glob_chars = options.pattern.contains('*')
            || options.pattern.contains('?')
            || options.pattern.contains('[')
            || options.pattern.contains('{');

        if has_glob_chars {
            globset::Glob::new(&options.pattern)
                .ok()
                .map(|g| g.compile_matcher())
        } else {
            None // Fall through to substring matching in visitor
        }
    };
    let pattern_lower = options.pattern.to_lowercase();

    // Precompute smart case flag once (performance optimization)
    let is_pattern_lowercase = options.pattern.chars().all(|c| !c.is_uppercase());

    let max_results = options.max_results.unwrap_or(DEFAULT_MAX_RESULTS as u32) as usize;

    // Build type_changes for ripgrep
    let type_changes = build_type_changes(options);

    // Build LowArgs for type filtering
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

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - handles built-in types + file_pattern
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = FileSearchBuilder {
        glob_pattern,
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
