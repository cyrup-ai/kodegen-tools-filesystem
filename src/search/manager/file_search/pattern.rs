//! Intelligent pattern type detection for file search
//!
//! Detects whether a pattern is regex, glob, or plain substring based on syntax.
//! Priority order when detecting: Regex > Glob > Substring

use crate::search::types::PatternMode;

/// Detect pattern type from syntax, with optional user override
pub fn detect(pattern: &str, literal_search: bool, user_override: Option<PatternMode>) -> PatternMode {
    // User explicitly specified - honor it
    if let Some(mode) = user_override {
        return mode;
    }
    
    // literal_search forces substring
    if literal_search {
        return PatternMode::Substring;
    }
    
    // Check regex first (more specific patterns)
    if has_regex_syntax(pattern) {
        return PatternMode::Regex;
    }
    
    // Check glob patterns
    if has_glob_syntax(pattern) {
        return PatternMode::Glob;
    }
    
    // Default to substring
    PatternMode::Substring
}

/// Check for regex-only syntax markers
/// 
/// These are patterns that are unambiguously regex and NOT valid glob syntax:
/// - Anchors: ^, $
/// - Escape sequences: \., \d, \w, \s, \b, \n, \t, \[, \(
/// - Common regex patterns: .*, .+
/// - Groups: (?...) for lookahead/lookbehind/non-capturing
/// - Alternation: | outside of {} braces
/// - Quantified groups: [...]+ or (...)+ or )+
fn has_regex_syntax(p: &str) -> bool {
    // Anchors - unambiguous regex markers
    if p.starts_with('^') || p.ends_with('$') {
        return true;
    }
    
    // Escape sequences - unambiguous regex markers
    // Note: In Rust string, we check for literal backslash followed by char
    if p.contains("\\.") || p.contains("\\d") || p.contains("\\w") ||
       p.contains("\\s") || p.contains("\\b") || p.contains("\\n") ||
       p.contains("\\t") || p.contains("\\[") || p.contains("\\(") ||
       p.contains("\\)") || p.contains("\\{") || p.contains("\\}") {
        return true;
    }
    
    // Common regex patterns: .* and .+ (dot followed by quantifier)
    if p.contains(".*") || p.contains(".+") || p.contains(".?") {
        return true;
    }
    
    // Lookahead/lookbehind/non-capturing groups
    if p.contains("(?") {
        return true;
    }
    
    // Alternation outside braces
    if contains_alternation(p) {
        return true;
    }
    
    // Quantified groups: ]+ or )+ or ]* or )* or ]? or )?
    if contains_quantified_group(p) {
        return true;
    }
    
    // Repetition quantifiers: {n}, {n,}, {n,m} where n is a digit
    if contains_repetition_quantifier(p) {
        return true;
    }
    
    false
}

/// Check for glob-specific syntax
fn has_glob_syntax(p: &str) -> bool {
    // Recursive wildcard (glob-only, regex would use .*/)
    if p.contains("**") {
        return true;
    }
    
    // Brace expansion with comma (glob-only: {a,b,c})
    if has_brace_expansion(p) {
        return true;
    }
    
    // Standard glob metacharacters (but NOT regex-specific uses)
    // * alone or not preceded by . (avoiding .*)
    // ? alone or not preceded by ( (avoiding (?)
    // [ without preceding \ (avoiding \[)
    if p.contains('*') && !p.contains(".*") && !p.contains(".+") {
        return true;
    }
    
    if p.contains('?') && !p.contains("(?") && !p.contains(".?") {
        return true;
    }
    
    // Character class not preceded by backslash
    if p.contains('[') && !p.contains("\\[") {
        return true;
    }
    
    false
}

/// Check for | alternation outside of {} braces
fn contains_alternation(p: &str) -> bool {
    let mut brace_depth: u32 = 0;
    let mut prev_char = '\0';
    
    for c in p.chars() {
        match c {
            '{' if prev_char != '\\' => brace_depth += 1,
            '}' if prev_char != '\\' => brace_depth = brace_depth.saturating_sub(1),
            '|' if brace_depth == 0 && prev_char != '\\' => return true,
            _ => {}
        }
        prev_char = c;
    }
    false
}

/// Check for quantified groups: ]+ )+ ]* )* ]? )?
fn contains_quantified_group(p: &str) -> bool {
    let chars: Vec<char> = p.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        let c = chars[i];
        let next = chars[i + 1];
        if (c == ']' || c == ')') && (next == '+' || next == '*' || next == '?') {
            return true;
        }
    }
    false
}

/// Check for regex repetition quantifiers: {n}, {n,}, {n,m}
/// 
/// Distinguishes from glob brace expansion {a,b} by requiring digits
fn contains_repetition_quantifier(p: &str) -> bool {
    let chars: Vec<char> = p.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if chars[i] == '{' && i > 0 {
            // Check if preceded by something quantifiable (not start of pattern)
            let mut j = i + 1;
            let mut has_digit = false;
            let mut valid = true;
            
            while j < chars.len() && chars[j] != '}' {
                match chars[j] {
                    '0'..='9' => has_digit = true,
                    ',' => {} // comma is valid in both {n,m} and {n,}
                    _ => {
                        valid = false;
                        break;
                    }
                }
                j += 1;
            }
            
            // {n} or {n,} or {n,m} pattern (all digits and optional comma)
            if valid && has_digit && j < chars.len() && chars[j] == '}' {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Check for glob brace expansion: {a,b,c}
/// 
/// Distinguishes from regex {n,m} by requiring non-digit content
fn has_brace_expansion(p: &str) -> bool {
    let mut in_brace = false;
    let mut has_comma = false;
    let mut has_non_digit = false;
    
    for c in p.chars() {
        match c {
            '{' => {
                in_brace = true;
                has_comma = false;
                has_non_digit = false;
            }
            '}' if in_brace => {
                if has_comma && has_non_digit {
                    return true;
                }
                in_brace = false;
            }
            ',' if in_brace => has_comma = true,
            c if in_brace && !c.is_ascii_digit() && c != ',' => has_non_digit = true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_detection() {
        // Anchors
        assert_eq!(detect("^test", false, None), PatternMode::Regex);
        assert_eq!(detect("test$", false, None), PatternMode::Regex);
        
        // Escape sequences
        assert_eq!(detect(r".*\.md$", false, None), PatternMode::Regex);
        assert_eq!(detect(r"\d+", false, None), PatternMode::Regex);
        assert_eq!(detect(r"\w+", false, None), PatternMode::Regex);
        
        // Alternation
        assert_eq!(detect("foo|bar", false, None), PatternMode::Regex);
        
        // Quantified groups
        assert_eq!(detect("[a-z]+", false, None), PatternMode::Regex);
        assert_eq!(detect("(foo)+", false, None), PatternMode::Regex);
        
        // Repetition quantifiers
        assert_eq!(detect("a{3}", false, None), PatternMode::Regex);
        assert_eq!(detect("a{2,5}", false, None), PatternMode::Regex);
        
        // Common patterns
        assert_eq!(detect(".*", false, None), PatternMode::Regex);
        assert_eq!(detect(".+", false, None), PatternMode::Regex);
    }

    #[test]
    fn test_glob_detection() {
        assert_eq!(detect("*.md", false, None), PatternMode::Glob);
        assert_eq!(detect("**/*.rs", false, None), PatternMode::Glob);
        assert_eq!(detect("{src,lib}/*.rs", false, None), PatternMode::Glob);
        assert_eq!(detect("test?.txt", false, None), PatternMode::Glob);
        assert_eq!(detect("[abc].txt", false, None), PatternMode::Glob);
    }

    #[test]
    fn test_substring_detection() {
        assert_eq!(detect("test", false, None), PatternMode::Substring);
        assert_eq!(detect("README", false, None), PatternMode::Substring);
        assert_eq!(detect("file-name", false, None), PatternMode::Substring);
        assert_eq!(detect("config.json", false, None), PatternMode::Substring);
        
        // Edge cases that should NOT be regex
        assert_eq!(detect("C++", false, None), PatternMode::Substring);
        assert_eq!(detect("file+name", false, None), PatternMode::Substring);
    }

    #[test]
    fn test_literal_forces_substring() {
        assert_eq!(detect("*.md", true, None), PatternMode::Substring);
        assert_eq!(detect(r".*\.md$", true, None), PatternMode::Substring);
    }

    #[test]
    fn test_user_override() {
        // User can force any mode
        assert_eq!(detect("test", false, Some(PatternMode::Regex)), PatternMode::Regex);
        assert_eq!(detect(".*", false, Some(PatternMode::Glob)), PatternMode::Glob);
        assert_eq!(detect("*.md", false, Some(PatternMode::Substring)), PatternMode::Substring);
    }
}
