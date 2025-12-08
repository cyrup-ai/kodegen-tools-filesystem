//! Line ending handling utilities for `edit_block`
//!
//! Provides detection, normalization, and conversion of line endings (LF/CRLF/CR).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndingStyle {
    /// Unix: \n
    Lf,
    /// Windows: \r\n
    Crlf,
    /// Classic Mac: \r
    Cr,
}

impl LineEndingStyle {
    /// Get string representation
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
        }
    }

    /// Get platform default
    #[must_use]
    pub fn platform_default() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Crlf;

        #[cfg(not(target_os = "windows"))]
        return Self::Lf;
    }
}

/// Detect line ending style from first line ending found
///
/// Returns platform default if no line endings are found.
/// This is a fast, single-pass check that examines only the first line ending.
#[must_use]
pub fn detect_line_ending(text: &str) -> LineEndingStyle {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'\r' => {
                // Check if CRLF or standalone CR
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    return LineEndingStyle::Crlf;
                } else {
                    return LineEndingStyle::Cr;
                }
            }
            b'\n' => {
                return LineEndingStyle::Lf;
            }
            _ => continue,
        }
    }
    // No line endings found, use platform default
    LineEndingStyle::platform_default()
}

/// Normalize line endings to target style
///
/// # Arguments
/// * `text` - Input text with potentially mixed line endings
/// * `target` - Target line ending style
///
/// # Returns
/// String with all line endings normalized to `target` style
///
/// # Example
/// ```rust
/// let text = "line1\r\nline2\nline3\rline4";
/// let normalized = normalize_line_endings(text, LineEndingStyle::Lf);
/// assert_eq!(normalized, "line1\nline2\nline3\nline4");
/// ```
#[must_use]
pub fn normalize_line_endings(text: &str, target: LineEndingStyle) -> String {
    // Step 1: Normalize everything to LF
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    // Step 2: Convert to target style
    match target {
        LineEndingStyle::Lf => normalized,
        LineEndingStyle::Crlf => normalized.replace('\n', "\r\n"),
        LineEndingStyle::Cr => normalized.replace('\n', "\r"),
    }
}

/// Analysis of line endings in content
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct LineEndingAnalysis {
    /// Predominant style (majority wins)
    pub style: LineEndingStyle,
    /// Total line ending count
    pub total_count: usize,
    /// Whether multiple styles are present
    pub has_mixed: bool,
    /// CRLF count
    pub crlf_count: usize,
    /// LF count
    pub lf_count: usize,
    /// CR count
    pub cr_count: usize,
}

/// Analyze line ending distribution
///
/// Full scan to detect mixed line endings.
/// Matches Desktop Commander implementation at lineEndingHandler.ts:45-90
#[must_use]
#[allow(dead_code)]
pub fn analyze_line_endings(content: &str) -> LineEndingAnalysis {
    let bytes = content.as_bytes();
    let mut crlf_count = 0;
    let mut lf_count = 0;
    let mut cr_count = 0;

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    crlf_count += 1;
                    i += 2; // Skip both \r and \n
                } else {
                    cr_count += 1;
                    i += 1;
                }
            }
            b'\n' => {
                lf_count += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Determine predominant style (majority wins)
    let style = if crlf_count >= lf_count && crlf_count >= cr_count {
        LineEndingStyle::Crlf
    } else if lf_count >= cr_count {
        LineEndingStyle::Lf
    } else {
        LineEndingStyle::Cr
    };

    // Check for mixed line endings
    let used_styles = [crlf_count > 0, lf_count > 0, cr_count > 0]
        .iter()
        .filter(|&&x| x)
        .count();

    LineEndingAnalysis {
        style,
        total_count: crlf_count + lf_count + cr_count,
        has_mixed: used_styles > 1,
        crlf_count,
        lf_count,
        cr_count,
    }
}
