//! ParallelVisitor trait implementation for file search

use super::{CompiledPattern, FileSearchVisitor};
use super::matching;
use crate::search::types::{CaseMode, SearchResult, SearchResultType};
use ignore::{DirEntry, ParallelVisitor};
use std::sync::atomic::Ordering;

impl ParallelVisitor for FileSearchVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Debug: log every entry we receive
        match &entry {
            Ok(e) => log::debug!("FileSearchVisitor::visit: entry={}", e.path().display()),
            Err(e) => log::debug!("FileSearchVisitor::visit: error={}", e),
        }

        // Fast check - if another thread already found exact match, quit immediately
        if self.early_termination && self.early_term_triggered.load(Ordering::Acquire) {
            self.flush_buffer();
            return ignore::WalkState::Quit;
        }

        // Check if we've reached max results
        if self.total_matches.load(Ordering::SeqCst) >= self.max_results {
            self.flush_buffer();
            return ignore::WalkState::Quit;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                self.track_error(&err);
                return ignore::WalkState::Continue;
            }
        };

        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let pattern_type = match &self.compiled_pattern {
            CompiledPattern::Regex(_) => "regex",
            CompiledPattern::Glob(_) => "glob",
            CompiledPattern::Substring => "substring",
        };
        log::debug!(
            "FileSearchVisitor: file_name='{}', pattern='{}', pattern_type={}, word_boundary={}",
            file_name,
            self.pattern,
            pattern_type,
            self.word_boundary
        );

        // Check if filename matches pattern based on compiled pattern type
        let matches = match &self.compiled_pattern {
            CompiledPattern::Regex(regex) => regex.is_match(file_name),
            CompiledPattern::Glob(glob) => glob.is_match(file_name),
            CompiledPattern::Substring => {
                if self.word_boundary {
                    // Word boundary: pattern must be surrounded by word boundaries
                    // Boundaries are: '.', '-', '_', '/', or start/end of string
                    matching::matches_with_word_boundary(
                        file_name,
                        &self.pattern,
                        &self.pattern_lower,
                        self.case_mode,
                        self.is_pattern_lowercase,
                    )
                } else {
                    // Substring match (current behavior)
                    let file_name_lower = file_name.to_lowercase();
                    match self.case_mode {
                        CaseMode::Insensitive => file_name_lower.contains(&self.pattern_lower),
                        CaseMode::Smart => {
                            // Smart: case-insensitive if pattern is all lowercase
                            if self.is_pattern_lowercase {
                                file_name_lower.contains(&self.pattern_lower)
                            } else {
                                file_name.contains(&self.pattern)
                            }
                        }
                        CaseMode::Sensitive => file_name.contains(&self.pattern),
                    }
                }
            }
        };

        if matches {
            // Check for exact match BEFORE reserving slot
            let is_exact = self.is_exact_match(file_name);
            
            if self.early_termination && is_exact {
                // Try to atomically claim "first exact match" status
                if self.early_term_triggered
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
                    .is_ok()
                {
                    // We're the FIRST thread to find exact match
                    // Reserve slot and add result
                    match self
                        .total_matches
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                            if current < self.max_results {
                                Some(current + 1)
                            } else {
                                None
                            }
                        }) {
                        Ok(_) => {
                            // Increment total_files for file search (each match IS a file)
                            self.total_files.fetch_add(1, Ordering::SeqCst);

                            // Collect metadata
                            let entry_metadata = entry.metadata().ok();
                            let modified = entry_metadata.as_ref().and_then(|m| m.modified().ok());
                            let accessed = entry_metadata.as_ref().and_then(|m| m.accessed().ok());
                            let created = entry_metadata.as_ref().and_then(|m| m.created().ok());

                            let search_result = SearchResult {
                                file: path.display().to_string(),
                                line: None,
                                r#match: None,
                                r#type: SearchResultType::File,
                                is_context: false,
                                is_binary: None,
                                binary_suppressed: None,
                                modified,
                                accessed,
                                created,
                            };

                            self.add_result(search_result);

                            // Flush and quit immediately
                            self.flush_buffer();
                            return ignore::WalkState::Quit;
                        }
                        Err(_) => {
                            // Max results reached - quit
                            self.flush_buffer();
                            return ignore::WalkState::Quit;
                        }
                    }
                } else {
                    // Another thread beat us to exact match - just quit
                    self.flush_buffer();
                    return ignore::WalkState::Quit;
                }
            }
            
            // Not exact match or early_termination disabled - normal flow
            match self
                .total_matches
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    if current < self.max_results {
                        Some(current + 1)
                    } else {
                        None
                    }
                }) {
                Ok(_) => {
                    // Increment total_files for file search (each match IS a file)
                    self.total_files.fetch_add(1, Ordering::SeqCst);

                    // Collect metadata and add result (same as before)
                    let entry_metadata = entry.metadata().ok();
                    let modified = entry_metadata.as_ref().and_then(|m| m.modified().ok());
                    let accessed = entry_metadata.as_ref().and_then(|m| m.accessed().ok());
                    let created = entry_metadata.as_ref().and_then(|m| m.created().ok());

                    let search_result = SearchResult {
                        file: path.display().to_string(),
                        line: None,
                        r#match: None,
                        r#type: SearchResultType::File,
                        is_context: false,
                        is_binary: None,
                        binary_suppressed: None,
                        modified,
                        accessed,
                        created,
                    };

                    self.add_result(search_result);
                }
                Err(_) => {
                    // Limit reached - quit searching
                    self.flush_buffer();
                    return ignore::WalkState::Quit;
                }
            }
        }

        ignore::WalkState::Continue
    }
}
