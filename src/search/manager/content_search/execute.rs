//! Main execute function for content search

use super::super::super::types::{ReturnMode, SearchResult};
use super::ContentSearchBuilder;
use ignore::WalkBuilder;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Execute content search using grep libraries with parallel directory traversal
pub(in super::super) fn execute(
    options: &super::super::super::types::SearchSessionOptions,
    root: &std::path::PathBuf,
    ctx: &mut super::super::context::SearchContext,
) {
    // Build LowArgs from SearchSessionOptions
    use super::super::super::rg::flags::hiargs::HiArgs;
    use super::super::super::rg::flags::lowargs::{
        BinaryMode as RgBinaryMode, BoundaryMode as RgBoundaryMode, ContextMode, EncodingMode,
        LowArgs, Mode, PatternSource, SearchMode,
    };
    use super::super::super::types::{BinaryMode, BoundaryMode};
    use super::super::utils::{build_type_changes, convert_case_mode};

    let mut context = ContextMode::default();

    // Set -C (context) first
    if options.context > 0 {
        context.set_both(options.context as usize);
    }

    // Override with -B (before_context) if specified
    if let Some(before) = options.before_context {
        context.set_before(before as usize);
    }

    // Override with -A (after_context) if specified
    if let Some(after) = options.after_context {
        context.set_after(after as usize);
    }

    // Convert MCP type params to ripgrep TypeChange format
    let type_changes = build_type_changes(options);

    // Map MCP BinaryMode to ripgrep's internal BinaryMode
    // Matches ripgrep's --binary and -a/--text flags
    let binary_mode = match options.binary_mode {
        BinaryMode::Auto => RgBinaryMode::Auto, // Default: skip binaries
        BinaryMode::Binary => RgBinaryMode::SearchAndSuppress, // --binary: search but suppress
        BinaryMode::Text => RgBinaryMode::AsText, // -a/--text: treat as text
    };

    // Convert encoding string to EncodingMode for rg
    let encoding_mode = match options.encoding.as_deref() {
        None | Some("auto") => EncodingMode::Auto,
        Some("none") => EncodingMode::Disabled,
        Some(enc_str) => match grep::searcher::Encoding::new(enc_str) {
            Ok(enc) => EncodingMode::Some(enc),
            Err(e) => {
                log::warn!("Invalid encoding '{enc_str}': {e}, using auto");
                EncodingMode::Auto
            }
        },
    };

    log::debug!(
        "content_search: case_mode from options = {:?}",
        options.case_mode
    );
    let mut low_args = LowArgs {
        patterns: vec![PatternSource::Regexp(options.pattern.clone())],
        positional: vec![root.as_os_str().to_owned()],
        case: convert_case_mode(options.case_mode),
        boundary: options.boundary_mode.map(|mode| match mode {
            BoundaryMode::Word => RgBoundaryMode::Word,
            BoundaryMode::Line => RgBoundaryMode::Line,
        }),
        fixed_strings: options.literal_search,
        context,
        max_count: if ctx.return_only == ReturnMode::Paths {
            Some(1) // Limit to 1 match per file for Paths mode (optimization)
        } else {
            options.max_results.map(u64::from)
        },
        max_filesize: options.max_filesize,
        max_depth: options.max_depth,
        hidden: options.include_hidden,
        line_number: Some(true),
        invert_match: options.invert_match,
        mode: Mode::Search(SearchMode::Json),
        only_matching: options.only_matching,
        type_changes,
        multiline: options.multiline,
        multiline_dotall: options.multiline,
        binary: binary_mode,
        encoding: encoding_mode,
        engine: convert_engine_choice(options.engine),
        // Match ripgrep's --no-ignore flag behavior exactly
        no_ignore_vcs: options.no_ignore,
        no_ignore_exclude: options.no_ignore,
        no_ignore_global: options.no_ignore,
        no_ignore_parent: options.no_ignore,
        no_ignore_dot: options.no_ignore,
        ..Default::default()
    };

    // Bridge SearchMode to ReturnMode for ripgrep CLI compatibility
    // Maps ripgrep's SearchMode enum variants to MCP's ReturnMode
    let effective_return_mode = match low_args.mode {
        Mode::Search(SearchMode::Count | SearchMode::CountMatches) => {
            // -c/--count flag
            ReturnMode::Counts
        }
        _ => {
            // For Standard and Json modes, use the MCP-provided return mode
            ctx.return_only
        }
    };

    // Update max_count based on effective return mode
    if effective_return_mode == ReturnMode::Paths && low_args.max_count.is_none() {
        // Optimization: stop after first match per file for Paths mode
        low_args.max_count = Some(1);
    }

    // Build HiArgs ONCE (expensive config processing)
    let hi_args = match HiArgs::from_low_args(low_args) {
        Ok(h) => h,
        Err(e) => {
            *ctx.error.blocking_write() = Some(format!("Failed to build HiArgs: {e}"));
            *ctx.is_error.blocking_write() = true;
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Build matcher ONCE (expensive: regex/PCRE2 compilation)
    let matcher = match hi_args.matcher() {
        Ok(m) => m,
        Err(e) => {
            *ctx.error.blocking_write() = Some(format!("Failed to build matcher: {e}"));
            *ctx.is_error.blocking_write() = true;
            ctx.is_complete.store(true, Ordering::Release);
            return;
        }
    };

    // Wrap HiArgs in Arc for cheap sharing across threads
    let hi_args = Arc::new(hi_args);

    // Build directory walker with gitignore support and parallel traversal
    let mut walker = WalkBuilder::new(root);
    super::super::utils::configure_walker(&mut walker, &hi_args);

    // Use HiArgs.types() - includes built-in types + file_pattern + type/type_not
    walker.types(hi_args.types().clone());

    // Build the parallel visitor
    let mut builder = ContentSearchBuilder {
        hi_args,
        matcher,
        max_results: options.max_results.map(|m| m as usize),
        return_only: effective_return_mode,
        results: Arc::clone(&ctx.results),
        total_matches: Arc::clone(&ctx.total_matches),
        total_files: Arc::clone(&ctx.total_files),
        last_read_time_atomic: Arc::clone(&ctx.last_read_time_atomic),
        cancellation_rx: ctx.cancellation_rx.clone(),
        first_result_tx: ctx.first_result_tx.clone(),
        was_incomplete: Arc::clone(&ctx.was_incomplete),
        error_count: Arc::clone(&ctx.error_count),
        errors: Arc::clone(&ctx.errors),
        seen_files: Arc::clone(&ctx.seen_files),
        file_counts: Arc::clone(&ctx.file_counts),
        start_time: ctx.start_time,
    };

    // Execute parallel search
    walker.build_parallel().visit(&mut builder);

    // Finalize Counts mode - convert HashMap counts to SearchResults
    if effective_return_mode == ReturnMode::Counts {
        use super::super::super::types::SearchResultType;

        // Phase 1: Build results Vec while holding only read lock on file_counts
        let results_to_add: Vec<SearchResult> = {
            let counts = ctx.file_counts.blocking_read();
            let mut results = Vec::with_capacity(counts.len());

            for (file, data) in counts.iter() {
                results.push(SearchResult {
                    file: file.clone(),
                    line: Some(data.count as u32), // Count stored in line field
                    r#match: None,
                    r#type: SearchResultType::Content,
                    is_context: false,
                    is_binary: None,
                    binary_suppressed: None,
                    modified: data.modified,
                    accessed: data.accessed,
                    created: data.created,
                });
            }

            results
        }; // Read lock on file_counts released here

        // Phase 2: Swap into results with brief write lock
        {
            let mut results_guard = ctx.results.blocking_write();

            // CountPerFile accumulates in file_counts HashMap, not results vec
            debug_assert!(
                results_guard.is_empty(),
                "CountPerFile: results vec should be empty before finalization"
            );

            *results_guard = results_to_add;

            // Update total_matches to reflect number of unique files with counts
            // In CountPerFile mode, total_matches should reflect unique file count for API consistency
            ctx.total_matches
                .store(ctx.total_files.load(Ordering::SeqCst), Ordering::SeqCst);
        } // Write lock on results released here
    }

    // Log error summary if any errors occurred
    let error_count = ctx.error_count.load(Ordering::SeqCst);
    if error_count > 0 {
        log::info!(
            "Content search completed with {} errors. Pattern: '{}', Path: {}",
            error_count,
            options.pattern,
            root.display()
        );
    }

    // Mark complete
    ctx.is_complete.store(true, Ordering::Release);
}

/// Convert MCP API `Engine` to ripgrep's internal `Engine`
fn convert_engine_choice(
    engine: super::super::super::types::Engine,
) -> super::super::super::rg::flags::lowargs::Engine {
    use super::super::super::rg::flags::lowargs::Engine as RG;
    use super::super::super::types::Engine as MCP;

    match engine {
        MCP::Auto => RG::Auto,
        MCP::Rust => RG::Default,
        MCP::PCRE2 => RG::PCRE2,
    }
}
