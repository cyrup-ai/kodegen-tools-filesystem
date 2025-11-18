//! ParallelVisitor implementation for ContentSearchVisitor

use super::super::super::types::{ReturnMode, SearchResult};
use super::ContentSearchVisitor;
use ignore::{DirEntry, ParallelVisitor};
use std::sync::atomic::Ordering;

impl ParallelVisitor for ContentSearchVisitor {
    fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        use super::super::super::rg::json_output::parse_json_buffer;

        log::debug!("ContentSearchVisitor::visit() called");

        // Check for cancellation
        if *self.cancellation_rx.borrow() {
            self.flush_buffer();
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }

        // Check if we've reached max results (mode-aware)
        if let Some(max) = self.max_results {
            let current_count = match self.return_only {
                ReturnMode::Counts => {
                    // In Counts mode, limit by unique files, not matches
                    self.total_files.load(Ordering::SeqCst)
                }
                _ => {
                    // In Matches/Paths modes, limit by matches
                    self.total_matches.load(Ordering::SeqCst)
                }
            };

            if current_count >= max {
                self.flush_buffer();
                return ignore::WalkState::Quit;
            }
        }

        // Track directory traversal errors
        if let Err(ref err) = entry {
            self.track_error(err);
            return ignore::WalkState::Continue;
        }

        // Extract Ok value - safe because we checked for Err above
        let dent = match entry {
            Ok(d) => d,
            Err(_) => unreachable!("Error case handled above"),
        };

        // Collect file metadata before searching (for sorting support)
        let file_metadata = dent.metadata().ok();
        let modified = file_metadata.as_ref().and_then(|m| m.modified().ok());
        let accessed = file_metadata.as_ref().and_then(|m| m.accessed().ok());
        let created = file_metadata.as_ref().and_then(|m| m.created().ok());

        // Build haystack from directory entry
        if let Some(haystack) = self.haystack_builder.build(dent) {
            log::debug!("Haystack built for file: {:?}", haystack.path());
            // Execute ripgrep search using full stack
            match self.worker.search(&haystack) {
                Ok(_search_result) => {
                    log::debug!("Search executed successfully");
                    // Parse JSON Lines to SearchResult
                    let results_parsed = {
                        // Access JSON buffer from printer (mutable borrow scope)
                        let buffer = self.worker.printer().get_mut();
                        log::debug!("JSON buffer length: {} bytes", buffer.len());
                        if log::log_enabled!(log::Level::Debug) && !buffer.is_empty() {
                            log::debug!("JSON buffer content: {}", String::from_utf8_lossy(buffer));
                        }
                        let parsed = parse_json_buffer(buffer);
                        buffer.clear(); // Clear immediately after parsing
                        parsed
                    }; // Mutable borrow of self.worker released here

                    if let Ok(mut results) = results_parsed {
                        log::debug!("Parsed {} results from JSON", results.len());
                        // Attach file metadata to all results from this file
                        for result in &mut results {
                            result.modified = modified;
                            result.accessed = accessed;
                            result.created = created;
                        }

                        for (i, result) in results.into_iter().enumerate() {
                            // Check cancellation every 100 results to balance responsiveness vs overhead
                            if i % 100 == 0 && *self.cancellation_rx.borrow() {
                                self.flush_buffer();
                                *self.was_incomplete.blocking_write() = true;
                                return ignore::WalkState::Quit;
                            }

                            // Mode-first branching: check return mode BEFORE reservation
                            match self.return_only {
                                ReturnMode::Matches => {
                                    // Matches mode: Always adds result
                                    // Reserve slot, then add
                                    match self.total_matches.fetch_update(
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                        |current| {
                                            if let Some(max) = self.max_results {
                                                if current < max {
                                                    Some(current + 1)
                                                } else {
                                                    None
                                                }
                                            } else {
                                                Some(current + 1)
                                            }
                                        },
                                    ) {
                                        Ok(_) => {
                                            // Use buffered approach for better performance
                                            self.add_result(result);
                                            self.maybe_update_last_read_time();
                                        }
                                        Err(_) => {
                                            // Limit reached
                                            self.flush_buffer();
                                            return ignore::WalkState::Quit;
                                        }
                                    }
                                }

                                ReturnMode::Paths => {
                                    // Paths mode: Deduplicate BEFORE reserving
                                    let mut seen = self.seen_files.blocking_write();
                                    if !seen.contains(&result.file) {
                                        // File not seen yet - try to reserve
                                        match self.total_matches.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |current| {
                                                if let Some(max) = self.max_results {
                                                    if current < max {
                                                        Some(current + 1)
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    Some(current + 1)
                                                }
                                            },
                                        ) {
                                            Ok(_) => {
                                                // Reserved successfully - mark as seen
                                                seen.insert(result.file.clone());
                                                drop(seen); // Release lock before next operation

                                                // Add deduplicated result
                                                let file_result = SearchResult {
                                                    file: result.file,
                                                    line: None,
                                                    r#match: None,
                                                    r#type: result.r#type,
                                                    is_context: false,
                                                    is_binary: result.is_binary,
                                                    binary_suppressed: result.binary_suppressed,
                                                    modified: result.modified,
                                                    accessed: result.accessed,
                                                    created: result.created,
                                                };
                                                // Use buffered approach for better performance
                                                self.add_result(file_result);
                                                self.maybe_update_last_read_time();
                                            }
                                            Err(_) => {
                                                // Hit limit - quit immediately
                                                drop(seen);
                                                self.flush_buffer();
                                                return ignore::WalkState::Quit;
                                            }
                                        }
                                    }
                                    // else: already seen this file, skip entirely
                                }

                                ReturnMode::Counts => {
                                    // Counts mode: Use total_files for limiting
                                    // DO NOT touch total_matches during search
                                    // (finalization at line 604 will set total_matches = total_files)

                                    // ✅ FIX: Acquire write lock FIRST, check and reserve atomically
                                    let mut counts = self.file_counts.blocking_write();

                                    // Check if this is a new file (inside write lock)
                                    if !counts.contains_key(&result.file) {
                                        // New file - try to reserve a slot in total_files
                                        match self.total_files.fetch_update(
                                            Ordering::SeqCst,
                                            Ordering::SeqCst,
                                            |current| {
                                                if let Some(max) = self.max_results {
                                                    if current < max {
                                                        Some(current + 1) // Reserve slot for this file
                                                    } else {
                                                        None // Hit limit - reject
                                                    }
                                                } else {
                                                    Some(current + 1) // No limit
                                                }
                                            },
                                        ) {
                                            Ok(_) => {
                                                // Successfully reserved - insert new file
                                                counts.insert(
                                                    result.file.clone(),
                                                    super::super::super::types::FileCountData {
                                                        count: 1,
                                                        modified: result.modified,
                                                        accessed: result.accessed,
                                                        created: result.created,
                                                    },
                                                );

                                                // Update timestamp
                                                let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
                                                self.last_read_time_atomic
                                                    .store(elapsed_micros, Ordering::Relaxed);
                                            }
                                            Err(_) => {
                                                // Hit file limit - stop search immediately
                                                drop(counts);
                                                self.flush_buffer();
                                                return ignore::WalkState::Quit;
                                            }
                                        }
                                    } else {
                                        // Existing file - just increment its match count (no limit check needed)
                                        if let Some(data) = counts.get_mut(&result.file) {
                                            data.count += 1;

                                            // Update timestamp
                                            let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
                                            self.last_read_time_atomic
                                                .store(elapsed_micros, Ordering::Relaxed);
                                        }
                                    }
                                    // Write lock released here automatically
                                }
                            }
                        }
                    } else if let Err(e) = results_parsed {
                        log::error!(
                            "JSON parsing error for {}: {}",
                            haystack.path().display(),
                            e
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Search error for {}: {}", haystack.path().display(), e);
                }
            }
        }

        ignore::WalkState::Continue
    }
}
