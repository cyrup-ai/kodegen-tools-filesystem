//! Pattern matching logic for file search

use crate::search::types::CaseMode;

/// Check if pattern matches with word boundaries
///
/// Word boundaries are: '.', '-', '_', '/', or start/end of string
/// E.g., "lib" matches "lib.rs" but not "libtest.rs"
pub(super) fn matches_with_word_boundary(
    file_name: &str,
    pattern: &str,
    pattern_lower: &str,
    case_mode: CaseMode,
    is_pattern_lowercase: bool,
) -> bool {
    /// Check if character is a word boundary separator
    fn is_boundary(c: char) -> bool {
        matches!(c, '.' | '-' | '_' | '/')
    }

    // Determine comparison strings based on case mode
    let (search_in, search_for) = match case_mode {
        CaseMode::Insensitive => (file_name.to_lowercase(), pattern_lower.to_string()),
        CaseMode::Smart => {
            if is_pattern_lowercase {
                (file_name.to_lowercase(), pattern_lower.to_string())
            } else {
                (file_name.to_string(), pattern.to_string())
            }
        }
        CaseMode::Sensitive => (file_name.to_string(), pattern.to_string()),
    };

    // Find all occurrences of the pattern
    let mut start = 0;
    while let Some(pos) = search_in[start..].find(&search_for) {
        let match_pos = start + pos;
        let match_end = match_pos + search_for.len();

        // Check if match is at start or preceded by boundary
        let before_ok = match_pos == 0 || {
            search_in[..match_pos]
                .chars()
                .last()
                .is_some_and(is_boundary)
        };

        // Check if match is at end or followed by boundary
        let after_ok = match_end == search_in.len() || {
            search_in[match_end..]
                .chars()
                .next()
                .is_some_and(is_boundary)
        };

        // If both boundaries are satisfied, we have a match
        if before_ok && after_ok {
            return true;
        }

        // Move past this occurrence and continue searching
        start = match_pos + 1;
    }

    false
}

/// Check if this is an exact match for a visitor
///
/// Returns true only when:
/// - Glob pattern has no wildcards and matches filename exactly, OR
/// - Literal pattern equals filename exactly (respecting `case_mode`)
pub(super) fn is_exact_match(
    glob_pattern: &Option<globset::GlobMatcher>,
    pattern: &str,
    case_mode: CaseMode,
    is_pattern_lowercase: bool,
    word_boundary: bool,
    file_name: &str,
) -> bool {
    // Word boundary mode: must match entire filename
    if word_boundary {
        if let Some(glob) = glob_pattern {
            return glob.is_match(file_name);
        }
        return match case_mode {
            CaseMode::Insensitive => file_name.eq_ignore_ascii_case(pattern),
            CaseMode::Smart => {
                if is_pattern_lowercase {
                    file_name.eq_ignore_ascii_case(pattern)
                } else {
                    file_name == pattern
                }
            }
            CaseMode::Sensitive => file_name == pattern,
        };
    }

    // Original logic: check for exact match in non-word-boundary mode
    if let Some(glob) = glob_pattern {
        // Check if glob pattern has no wildcards
        let has_wildcards = pattern.contains('*')
            || pattern.contains('?')
            || pattern.contains('[');

        // Not exact if pattern contains wildcards
        if has_wildcards {
            return false;
        }

        // Exact match if no wildcards and pattern matches
        glob.is_match(file_name)
    } else {
        // For literal/substring matching, exact means equality
        match case_mode {
            CaseMode::Insensitive => file_name.eq_ignore_ascii_case(pattern),
            CaseMode::Smart => {
                // Smart: case-insensitive if pattern is all lowercase
                if is_pattern_lowercase {
                    file_name.eq_ignore_ascii_case(pattern)
                } else {
                    file_name == pattern
                }
            }
            CaseMode::Sensitive => file_name == pattern,
        }
    }
}
