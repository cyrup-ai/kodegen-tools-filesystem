//! ParallelVisitor trait implementation for file search

use super::FileSearchVisitor;
use super::matching;
use crate::search::types::{CaseMode, SearchResult, SearchResultType};
use ignore::{DirEntry, ParallelVisitor};
use std::sync::atomic::Ordering;

impl ParallelVisitor for FileSearchVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Check for cancellation
        if *self.cancellation_rx.borrow() {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
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

        // Check if filename matches pattern
        // Try glob pattern first, then fall back to substring or word boundary matching
        let matches = if let Some(ref glob) = self.glob_pattern {
            glob.is_match(file_name)
        } else if self.word_boundary {
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
                            self.maybe_update_last_read_time();
                            
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
                    self.maybe_update_last_read_time();
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
