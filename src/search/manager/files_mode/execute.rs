//!
//! Execute function for files mode

use super::visitor_builder::FilesListerBuilder;
use super::super::context::SearchContext;
use super::super::config::DEFAULT_MAX_RESULTS;

use super::super::super::rg::flags::lowargs::CaseMode as RgCaseMode;
use super::super::super::rg::flags::{
    hiargs::HiArgs,
    lowargs::{LowArgs, Mode, TypeChange},
};
use super::super::super::types::SearchSessionOptions;

use ignore::WalkBuilder;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Execute files mode: list all files that would be searched
pub fn execute(options: &SearchSessionOptions, root: &Path, ctx: &mut SearchContext) {
    let max_results = options.max_results.unwrap_or(DEFAULT_MAX_RESULTS as u32) as usize;

    // Build type changes for filtering
    let mut type_changes = Vec::new();
    for type_name in &options.r#type {
        type_changes.push(TypeChange::Select {
            name: type_name.clone(),
        });
    }
    for type_name in &options.type_not {
        type_changes.push(TypeChange::Negate {
            name: type_name.clone(),
        });
    }

    // Build LowArgs for type filtering
    // Note: We use a dummy pattern since we're not actually searching
    let low_args = LowArgs {
        patterns: vec![],
        case: RgCaseMode::Sensitive,
        fixed_strings: false,
        hidden: options.include_hidden,
        invert_match: false,
        mode: Mode::Files,
        type_changes,
        // Match ripgrep's --no-ignore flag behavior exactly
        no_ignore_vcs: options.no_ignore,
        no_ignore_exclude: options.no_ignore,
        no_ignore_global: options.no_ignore,
        no_ignore_parent: options.no_ignore,
        no_ignore_dot: options.no_ignore,
        max_depth: options.max_depth,
        ..Default::default()
    };

    // Build HiArgs for types
    let hi_args = match HiArgs::from_low_args(low_args) {
        Ok(h) => Arc::new(h),
        Err(e) => {
            log::error!("Failed to build HiArgs: {e}");
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    super::super::utils::configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - handles built-in types
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = FilesListerBuilder::new(max_results, ctx);

    // Execute parallel walk
    walker.build_parallel().visit(&mut builder);

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "Files mode completed with {} errors. Path: {}",
            error_count,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete.store(true, Ordering::Release);
}
