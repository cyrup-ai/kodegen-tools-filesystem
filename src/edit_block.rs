use crate::{validate_path, display_path_relative_to_git_root};
use chrono::Utc;
use kodegen_mcp_schema::filesystem::{FsEditBlockArgs, FsEditBlockPromptArgs};
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_utils::char_analysis::CharCodeData;
use kodegen_utils::char_diff::CharDiff;
use kodegen_utils::edit_log::{EditBlockLogEntry, EditBlockResult, get_edit_logger};
use kodegen_utils::fuzzy_logger::{FuzzySearchLogEntry, get_logger};
use kodegen_utils::fuzzy_search::{get_similarity_ratio, recursive_fuzzy_index_of_with_defaults};
use kodegen_utils::line_endings::{detect_line_ending, normalize_line_endings};
use kodegen_utils::suggestions::{EditFailureReason, Suggestion, SuggestionContext};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::time::Instant;
use tokio::fs;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Safely count newlines before a byte index, adjusting for UTF-8 boundaries
///
/// The byte_index may fall inside a multi-byte UTF-8 character. This function
/// adjusts it to the nearest valid character boundary at or before the index,
/// then counts newlines in that prefix.
///
/// # Arguments
/// * `content` - The string to search in
/// * `byte_index` - Byte index from fuzzy search (may not be char-aligned)
///
/// # Returns
/// Line number (1-based) where the byte index occurs
fn count_lines_before_index(content: &str, byte_index: usize) -> usize {
    // Adjust to nearest valid UTF-8 character boundary at or before byte_index
    let safe_index = content.floor_char_boundary(byte_index);
    
    // Count newlines in the safe prefix and add 1 for 1-based line numbers
    content[..safe_index].matches('\n').count() + 1
}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct EditBlockTool {
    config_manager: kodegen_config_manager::ConfigManager,
}

impl EditBlockTool {
    #[must_use]
    pub fn new(config_manager: kodegen_config_manager::ConfigManager) -> Self {
        Self { config_manager }
    }

    fn build_comprehensive_prompt(&self, show_advanced: bool) -> String {
        let mut content = String::from(
            "The edit_block tool performs surgical text replacements with exact string matching (not regex). \
             It's designed for precise, unambiguous edits with sophisticated error handling and fuzzy fallback.\n\n\
             ## What edit_block Does\n\n\
             - Performs EXACT string matching (not regex pattern matching)\n\
             - Replaces ALL occurrences of old_string with new_string\n\
             - Verifies replacement count matches your expectations\n\
             - Falls back to fuzzy search when exact match not found\n\
             - Provides character-level analysis of differences\n\n\
             ## Basic Usage\n\n\
             1. Single replacement (default expected_replacements: 1):\n\
                fs_edit_block({\n\
                  \"path\": \"src/config.rs\",\n\
                  \"old_string\": \"const MAX_RETRIES: u32 = 3;\",\n\
                  \"new_string\": \"const MAX_RETRIES: u32 = 5;\"\n\
                })\n\n\
             2. Multiple replacements (specify exact count):\n\
                fs_edit_block({\n\
                  \"path\": \"tests/integration.rs\",\n\
                  \"old_string\": \"assert_eq!(result, true);\",\n\
                  \"new_string\": \"assert!(result);\",\n\
                  \"expected_replacements\": 12\n\
                })\n\n\
             3. Unknown count (use 0 to accept any number):\n\
                fs_edit_block({\n\
                  \"path\": \"README.md\",\n\
                  \"old_string\": \"v1.2.3\",\n\
                  \"new_string\": \"v1.3.0\",\n\
                  \"expected_replacements\": 0\n\
                })\n\n\
             ## expected_replacements Parameter\n\n\
             This parameter helps catch unexpected matches:\n\
             - Default: 1 (expects exactly one occurrence)\n\
             - If actual count differs: tool still succeeds but returns WARNING\n\
             - Set to 0: accepts any count without warning\n\
             - Use when unsure: prevents silent multi-replacements\n\n\
             ## Fuzzy Search Fallback\n\n\
             When exact match not found, tool automatically:\n\
             1. Performs fuzzy search to find similar text\n\
             2. Calculates similarity score (0.0-1.0)\n\
             3. If above threshold: shows character-level diff and rejects\n\
             4. If below threshold: shows best match found and suggests fixes\n\
             This helps diagnose whitespace issues, encoding drift, typos.\n\n\
             ## Best Practices\n\n\
             - Include surrounding context to make old_string unique\n\
             - Always specify expected_replacements when you know the count\n\
             - Check response metadata to verify replacement count\n\
             - For multiple similar edits, make separate calls with context\n\
             - Use expected_replacements: 0 only when count truly unknown\n\n\
             ## Common Patterns\n\n\
             1. Updating import statement:\n\
                old_string: \"use std::collections::HashMap;\"\n\
                new_string: \"use std::collections::{HashMap, HashSet};\"\n\n\
             2. Changing function signature with context:\n\
                old_string: \"pub fn process(data: &str) -> Result<String> {\"\n\
                new_string: \"pub fn process(data: &str) -> Result<String, Error> {\"\n\n\
             3. Config value update (multiple occurrences):\n\
                old_string: \"timeout_ms: 5000\"\n\
                new_string: \"timeout_ms: 10000\"\n\
                expected_replacements: 3\n\n\
             4. Refactoring variable name with unique context:\n\
                old_string: \"let result = validate_input(&data);\"\n\
                new_string: \"let validation_result = validate_input(&data);\"\n\n\
             ## Error Handling\n\n\
             - No exact match: fuzzy search activates, shows diff if similar enough\n\
             - Count mismatch: succeeds with warning showing expected vs actual\n\
             - Empty old_string: rejected (cannot search for empty string)\n\
             - Identical old/new: rejected (no-op)\n\
             - Invalid path: rejected with path validation error\n\n\
             ## Important Gotchas\n\n\
             - NOT regex: special characters like . * + ? [ ] are treated literally\n\
             - Replaces ALL occurrences: each call affects the entire file\n\
             - Not idempotent: running twice changes file differently each time\n\
             - Line endings: tool auto-normalizes (CRLF vs LF) transparently\n\
             - UTF-8: multi-byte character boundaries handled automatically"
        );

        if show_advanced {
            content.push_str(
                "\n\n## Advanced Topics\n\n\
                 ### Fuzzy Search Configuration\n\n\
                 The similarity threshold is configurable via config_manager:\n\
                 - Default threshold: typically 0.6 (60% similarity)\n\
                 - Matches below threshold: rejected with suggestions\n\
                 - Matches above threshold: show diff and reject (prevents accidental bad edits)\n\n\
                 ### Character-Level Analysis\n\n\
                 When fuzzy match found, tool provides:\n\
                 - Unicode character code comparison\n\
                 - Whitespace-only change detection\n\
                 - Character-by-character diff display\n\
                 - UTF-8 boundary safety (adjusts to valid char boundaries)\n\n\
                 ### Logging System\n\n\
                 All edit operations are logged:\n\
                 - Exact matches: logged with count and timing\n\
                 - Fuzzy attempts: logged with similarity scores\n\
                 - Log location included in fuzzy error messages\n\
                 - Fire-and-forget async logging (never blocks execution)\n\n\
                 ### Performance Considerations\n\n\
                 - Large files: entire content loaded into memory\n\
                 - Fuzzy search: expensive for large files, only runs on no exact match\n\
                 - Line limit warnings: triggered if search/replace text exceeds configured limit\n\
                 - Execution time: included in response metadata"
            );
        }

        content
    }

    fn build_fuzzy_search_prompt(&self, show_advanced: bool) -> String {
        let mut content = String::from(
            "The edit_block tool has sophisticated fuzzy search fallback that activates when exact match fails.\n\n\
             ## When Fuzzy Search Activates\n\n\
             Fuzzy search runs automatically when:\n\
             - No exact match for old_string found in file\n\
             - Helps diagnose typos, whitespace issues, encoding differences\n\n\
             ## How Similarity Threshold Works\n\n\
             The tool calculates a similarity score (0.0-1.0) between your search string and the best match found:\n\
             - 1.0 = perfect match (100% identical)\n\
             - 0.8 = high similarity (80% matching characters)\n\
             - 0.5 = moderate similarity (50% matching characters)\n\
             - 0.0 = completely different\n\n\
             ## Fuzzy Match Above Threshold\n\n\
             When similarity ≥ threshold (typically 0.6), tool REJECTS the edit and shows:\n\
             - Similarity score\n\
             - Character-level diff highlighting exact differences\n\
             - Line number where match was found\n\
             - Whether difference is whitespace-only\n\
             - Unicode character code analysis\n\n\
             Example error message:\n\
             ```\n\
             No exact match found. Fuzzy search found similar text with 78.3% similarity (line 42).\n\n\
             Character-level differences:\n\
             Search:  \"const MAX_SIZE: usize = 1024;\"\n\
             Found:   \"const MAX_SIZE: usize = 1024; \"\n\
                                                   ^\n\
             Note: Difference is whitespace only.\n\
             ```\n\n\
             ## Fuzzy Match Below Threshold\n\n\
             When similarity < threshold, tool shows:\n\
             - Best match found (even if poor)\n\
             - Similarity score\n\
             - Threshold value\n\
             - Suggestions for fixing the search string\n\n\
             ## Character-Level Diff Display\n\n\
             The diff shows:\n\
             - Exact position of differences (^ markers)\n\
             - Added characters (in Found but not Search)\n\
             - Removed characters (in Search but not Found)\n\
             - Whitespace differences highlighted\n\n\
             ## Whitespace Detection\n\n\
             Tool detects when ONLY difference is whitespace:\n\
             - Trailing spaces\n\
             - Tabs vs spaces\n\
             - Extra newlines\n\
             - Carriage returns (CRLF vs LF)\n\
             Especially useful for indentation issues!\n\n\
             ## Use Cases for Fuzzy Search\n\n\
             1. Diagnosing copy-paste errors:\n\
                - Invisible trailing whitespace\n\
                - Tab/space confusion\n\n\
             2. Finding encoding drift:\n\
                - Smart quotes vs straight quotes\n\
                - Em-dash vs hyphen\n\
                - Non-breaking spaces\n\n\
             3. Identifying typos:\n\
                - Misspelled variable names\n\
                - Missing/extra characters\n\n\
             4. Debugging failed edits:\n\
                - Shows what's actually in the file\n\
                - Highlights exact differences\n\n\
             ## Example Workflow\n\n\
             1. Attempt edit:\n\
                fs_edit_block({\n\
                  \"path\": \"config.rs\",\n\
                  \"old_string\": \"timeout: 5000\",\n\
                  \"new_string\": \"timeout: 10000\"\n\
                })\n\n\
             2. Get fuzzy match error showing file has \"timeout:  5000\" (extra space)\n\n\
             3. Fix search string and retry:\n\
                fs_edit_block({\n\
                  \"path\": \"config.rs\",\n\
                  \"old_string\": \"timeout:  5000\",\n\
                  \"new_string\": \"timeout: 10000\"\n\
                })"
        );

        if show_advanced {
            content.push_str(
                "\n\n## Advanced Details\n\n\
                 ### Similarity Algorithm\n\n\
                 Uses recursive fuzzy matching with defaults:\n\
                 - Character-by-character comparison\n\
                 - Handles insertions, deletions, substitutions\n\
                 - UTF-8 boundary aware (adjusts to valid character boundaries)\n\
                 - Returns best match in entire file\n\n\
                 ### Threshold Configuration\n\n\
                 Threshold is retrieved from config_manager:\n\
                 - Typically 0.6 (60% similarity)\n\
                 - Configurable per-installation\n\
                 - Higher threshold = more strict (fewer false positives)\n\
                 - Lower threshold = more lenient (more matches accepted)\n\n\
                 ### Performance\n\n\
                 - Fuzzy search is expensive (recursive algorithm)\n\
                 - Only runs when exact match fails\n\
                 - Execution time included in error message\n\
                 - All attempts logged to fuzzy search log\n\n\
                 ### Logging\n\n\
                 Each fuzzy attempt logs:\n\
                 - Search text and found text\n\
                 - Similarity score and threshold\n\
                 - Character-level diff\n\
                 - Execution time\n\
                 - File extension\n\
                 - Whether accepted or rejected\n\
                 Log path included in error messages for debugging."
            );
        }

        content
    }

    fn build_expected_replacements_prompt(&self, show_advanced: bool) -> String {
        let mut content = String::from(
            "The expected_replacements parameter is crucial for safe, verified edits.\n\n\
             ## Why expected_replacements Matters\n\n\
             It catches unexpected multi-matches:\n\
             - You think you're replacing 1 occurrence\n\
             - File actually has 5 occurrences\n\
             - Without verification: all 5 get replaced silently\n\
             - With expected_replacements: you get warned about the mismatch\n\n\
             ## How It Works\n\n\
             1. Tool counts actual occurrences of old_string\n\
             2. Compares actual count to expected_replacements\n\
             3. If match: success (no warning)\n\
             4. If mismatch: still succeeds BUT returns warning with details\n\n\
             ## Behavior on Mismatch\n\n\
             Tool does NOT fail on count mismatch:\n\
             - Replacement still happens (all occurrences replaced)\n\
             - Response includes success: true\n\
             - Response includes warning with expected vs actual count\n\
             - Response metadata includes matched_expected: false\n\n\
             ## Using expected_replacements = 0\n\n\
             Special value meaning \"accept any count\":\n\
             - No warning regardless of actual count\n\
             - Use when count is genuinely unknown\n\
             - Use for exploratory replacements\n\
             - Still safer than omitting the parameter (default is 1!)\n\n\
             ## Default Value\n\n\
             If you omit expected_replacements:\n\
             - Defaults to 1 (expects exactly one occurrence)\n\
             - If file has 0 or 2+ occurrences: you get a warning\n\
             - This default protects against accidental multi-replacements\n\n\
             ## Examples\n\n\
             1. Safe single replacement:\n\
                fs_edit_block({\n\
                  \"path\": \"config.rs\",\n\
                  \"old_string\": \"const VERSION: &str = \\\"1.0.0\\\";\",\n\
                  \"new_string\": \"const VERSION: &str = \\\"1.1.0\\\";\",\n\
                  \"expected_replacements\": 1\n\
                })\n\
                Result: Success if exactly 1 match, warning otherwise\n\n\
             2. Bulk refactoring:\n\
                fs_edit_block({\n\
                  \"path\": \"tests/mod.rs\",\n\
                  \"old_string\": \"use_legacy_api()\",\n\
                  \"new_string\": \"use_new_api()\",\n\
                  \"expected_replacements\": 15\n\
                })\n\
                Result: Success if exactly 15 matches, warning if different\n\n\
             3. Unknown count (exploratory):\n\
                fs_edit_block({\n\
                  \"path\": \"docs/README.md\",\n\
                  \"old_string\": \"http://old-domain.com\",\n\
                  \"new_string\": \"https://new-domain.com\",\n\
                  \"expected_replacements\": 0\n\
                })\n\
                Result: Always success, no warning regardless of count\n\n\
             ## Real-World Scenario\n\n\
             Problem: You want to replace a common word like \"data\" in a specific context.\n\n\
             Bad approach (risky):\n\
             fs_edit_block({\n\
               \"path\": \"processor.rs\",\n\
               \"old_string\": \"data\",\n\
               \"new_string\": \"input_data\",\n\
               \"expected_replacements\": 0\n\
             })\n\
             Risk: Replaces ALL 47 occurrences of \"data\", breaking unrelated code!\n\n\
             Good approach (safe):\n\
             fs_edit_block({\n\
               \"path\": \"processor.rs\",\n\
               \"old_string\": \"fn process(data: &[u8])\",\n\
               \"new_string\": \"fn process(input_data: &[u8])\",\n\
               \"expected_replacements\": 1\n\
             })\n\
             Safe: Includes context, expects 1 match, warns if different\n\n\
             ## Response Format\n\n\
             On count mismatch, response includes:\n\
             - Content[0]: Human summary with warning note\n\
             - Content[1]: JSON with:\n\
               - success: true (edit still happened)\n\
               - replacements: actual count\n\
               - expected_replacements: what you specified\n\
               - matched_expected: false\n\
               - warning: descriptive message about mismatch\n\n\
             ## Best Practices\n\n\
             - Always specify expected_replacements when you know the count\n\
             - Use sufficient context to make old_string unique\n\
             - Check response metadata to verify count\n\
             - Use 0 only when count is truly unknown\n\
             - For critical edits: use expected_replacements: 1 with unique context"
        );

        if show_advanced {
            content.push_str(
                "\n\n## Advanced Details\n\n\
                 ### Count Verification Logic\n\n\
                 1. Tool uses str::matches() to count occurrences\n\
                 2. Compares count to expected_replacements\n\
                 3. If match: sets matched_expected: true in response\n\
                 4. If mismatch: sets matched_expected: false, adds warning\n\n\
                 ### Logging\n\n\
                 All edits logged with:\n\
                 - exact_match_count: actual occurrences found\n\
                 - expected_replacements: what you specified\n\
                 - Logs stored in edit_block log (fire-and-forget async)\n\n\
                 ### Special Case: expected_replacements = 0\n\n\
                 Implementation detail:\n\
                 - 0 means \"any count is acceptable\"\n\
                 - No warning generated regardless of actual count\n\
                 - matched_expected: true in response (since any count matches)\n\
                 - Useful for batch operations with unknown scope"
            );
        }

        content
    }

    fn build_line_endings_prompt(&self, show_advanced: bool) -> String {
        let mut content = String::from(
            "The edit_block tool handles line ending differences automatically and transparently.\n\n\
             ## Line Ending Types\n\n\
             Different operating systems use different line endings:\n\
             - Unix/Linux/macOS: LF (\\n) - single character\n\
             - Windows: CRLF (\\r\\n) - two characters\n\
             - Classic Mac: CR (\\r) - rare, mostly legacy\n\n\
             ## The Problem\n\n\
             When you copy text from a file for old_string:\n\
             - Your search string might have LF (\\n)\n\
             - File might have CRLF (\\r\\n)\n\
             - Exact string match would fail!\n\
             - Fuzzy search would show line-ending differences\n\n\
             ## The Solution\n\n\
             Tool automatically:\n\
             1. Detects file's line ending style (CRLF or LF)\n\
             2. Normalizes your old_string to match file's style\n\
             3. Normalizes your new_string to match file's style\n\
             4. Performs search/replace with normalized strings\n\
             5. File's line ending style is preserved\n\n\
             ## Transparent to You\n\n\
             You don't need to:\n\
             - Know file's line ending style\n\
             - Manually add \\r characters\n\
             - Convert line endings beforehand\n\
             - Handle different OS conventions\n\
             Tool handles all complexity internally!\n\n\
             ## When It Matters\n\n\
             1. Editing files from different OSes:\n\
                - Windows file checked into Git on Mac\n\
                - Cross-platform repository\n\
                - Files with mixed line endings\n\n\
             2. Copy-pasting search strings:\n\
                - Copy from Windows editor (CRLF)\n\
                - File on Linux system (LF)\n\
                - Tool normalizes automatically\n\n\
             3. Multi-line search/replace:\n\
                - old_string spans multiple lines\n\
                - Line endings must match file\n\
                - Normalization ensures match\n\n\
             ## Example\n\n\
             Your search string (LF line endings):\n\
             ```\n\
             fn main() {\\n    println!(\\\"Hello\\\");\\n}\n\
             ```\n\n\
             File has CRLF line endings:\n\
             ```\n\
             fn main() {\\r\\n    println!(\\\"Hello\\\");\\r\\n}\n\
             ```\n\n\
             Tool automatically:\n\
             1. Detects file uses CRLF\n\
             2. Converts your search string to CRLF\n\
             3. Finds exact match\n\
             4. Applies replacement\n\
             5. Preserves CRLF in output\n\n\
             ## Git and .gitattributes\n\n\
             Git can alter line endings on checkout:\n\
             - text=auto: Git normalizes to LF in repo, converts on checkout\n\
             - text eol=crlf: Force CRLF on checkout\n\
             - text eol=lf: Force LF on checkout\n\
             Tool reads actual file content (post-checkout), so it always matches.\n\n\
             ## Best Practice\n\n\
             You don't need to think about line endings!\n\
             Just use the tool normally:\n\
             - Copy text from file as-is\n\
             - Use it as old_string\n\
             - Tool handles normalization\n\
             - File's style is preserved"
        );

        if show_advanced {
            content.push_str(
                "\n\n## Advanced Details\n\n\
                 ### Line Ending Detection\n\n\
                 Tool scans file content:\n\
                 - Looks for CRLF (\\r\\n) sequences\n\
                 - If found: file uses CRLF style\n\
                 - If not found: file uses LF style\n\
                 - Classic CR (\\r alone) treated as LF\n\n\
                 ### Normalization Process\n\n\
                 For each string (old_string and new_string):\n\
                 1. Detect target style (from file)\n\
                 2. Replace all \\r\\n with \\n (normalize to LF)\n\
                 3. If target is CRLF: replace all \\n with \\r\\n\n\
                 4. Result: string matches file's line ending style\n\n\
                 ### Preservation\n\n\
                 After replacement:\n\
                 - File's line ending style unchanged\n\
                 - New content uses same style as original\n\
                 - Git won't see line ending changes\n\
                 - Diffs show only actual content changes\n\n\
                 ### Mixed Line Endings\n\n\
                 If file has mixed line endings:\n\
                 - Detection uses first CRLF found (or LF if none)\n\
                 - Normalization applies uniformly\n\
                 - May actually normalize mixed-ending files\n\
                 - Generally beneficial (fixes inconsistency)"
            );
        }

        content
    }

    fn build_edge_cases_prompt(&self, show_advanced: bool) -> String {
        let mut content = String::from(
            "The edit_block tool has strict validation and error handling for edge cases.\n\n\
             ## Empty Search String\n\n\
             Always rejected:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"file.txt\",\n\
               \"old_string\": \"\",\n\
               \"new_string\": \"something\"\n\
             })\n\
             Error: Empty search strings are not allowed.\n\
             ```\n\
             Reason: Replacing empty string would insert at every position!\n\n\
             ## Identical old_string and new_string\n\n\
             Always rejected:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"file.txt\",\n\
               \"old_string\": \"foo\",\n\
               \"new_string\": \"foo\"\n\
             })\n\
             Error: old_string and new_string are identical. No changes would be made.\n\
             ```\n\
             Reason: No-op, file wouldn't change anyway.\n\n\
             ## Multi-byte UTF-8 Characters\n\n\
             Tool handles automatically:\n\
             - Fuzzy search adjusts to character boundaries\n\
             - No risk of splitting multi-byte characters\n\
             - Emoji, Chinese, Arabic, etc. all safe\n\n\
             Example (safe):\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"messages.txt\",\n\
               \"old_string\": \"Status: ✓ Complete\",\n\
               \"new_string\": \"Status: ✗ Failed\"\n\
             })\n\
             ```\n\
             Checkmark (✓) is 3-byte UTF-8, tool handles correctly.\n\n\
             ## Character Encoding Mismatches\n\n\
             File must be valid UTF-8:\n\
             - Tool reads file as UTF-8 string\n\
             - If file has invalid UTF-8: read error\n\
             - If different encoding (Latin-1, etc.): may misinterpret\n\n\
             Detection:\n\
             - Fuzzy search shows character code differences\n\
             - Helps identify encoding issues\n\n\
             Solution:\n\
             - Convert file to UTF-8 first\n\
             - Use iconv, dos2unix, or editor conversion\n\n\
             ## Very Large Search/Replace Strings\n\n\
             Tool warns if exceeds line limit:\n\
             - Default limit: typically 500 lines\n\
             - Configurable via config_manager\n\
             - Warning included in response\n\
             - Edit still proceeds (not blocked)\n\n\
             Recommendation:\n\
             - Break large edits into smaller chunks\n\
             - Use multiple calls with specific context\n\
             - Reduces risk of unexpected matches\n\n\
             ## Path Safety\n\n\
             Tool validates paths:\n\
             - Expands ~ to home directory\n\
             - Resolves symlinks to real paths\n\
             - Checks against allowed directories (if configured)\n\
             - Rejects paths escaping allowed dirs\n\n\
             Example error:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"../../etc/passwd\",\n\
               \"old_string\": \"...\",\n\
               \"new_string\": \"...\"\n\
             })\n\
             Error: Path outside allowed directories.\n\
             ```\n\n\
             ## File Not Found\n\n\
             Rejected before any search:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"nonexistent.txt\",\n\
               \"old_string\": \"...\",\n\
               \"new_string\": \"...\"\n\
             })\n\
             Error: No such file or directory\n\
             ```\n\n\
             ## Permission Denied\n\n\
             Rejected if file not writable:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"/root/protected.txt\",\n\
               \"old_string\": \"...\",\n\
               \"new_string\": \"...\"\n\
             })\n\
             Error: Permission denied\n\
             ```\n\n\
             ## Regex Special Characters\n\n\
             NOT regex, so special chars are LITERAL:\n\
             ```\n\
             fs_edit_block({\n\
               \"path\": \"regex.txt\",\n\
               \"old_string\": \"data.*\",\n\
               \"new_string\": \"result.*\"\n\
             })\n\
             ```\n\
             This searches for literal string \"data.*\" (including the dot and asterisk),\n\
             NOT a regex pattern! This is exact string matching.\n\n\
             ## No Match Found\n\n\
             Triggers fuzzy search:\n\
             1. Tool attempts fuzzy search\n\
             2. Shows best match found (even if poor)\n\
             3. Shows similarity score\n\
             4. Provides character-level diff\n\
             5. Suggests fixes\n\
             6. Returns error (edit not applied)\n\n\
             ## Count Mismatch\n\n\
             Edit succeeds with warning:\n\
             - All matches replaced\n\
             - Response includes warning\n\
             - Metadata shows expected vs actual\n\
             - matched_expected: false\n\
             - You can check response to verify"
        );

        if show_advanced {
            content.push_str(
                "\n\n## Advanced Details\n\n\
                 ### UTF-8 Boundary Safety\n\n\
                 Function: count_lines_before_index()\n\
                 - Fuzzy search returns byte index (not char index)\n\
                 - Byte index might fall inside multi-byte char\n\
                 - Tool uses str::floor_char_boundary() to adjust\n\
                 - Ensures no invalid UTF-8 slicing\n\n\
                 ### Path Validation Process\n\n\
                 1. Expand ~ to home directory\n\
                 2. Resolve symlinks to canonical path\n\
                 3. Check against allowed_directories config\n\
                 4. Reject if outside allowed dirs\n\
                 5. Check file exists and is readable\n\
                 Uses validate_path() from filesystem helpers.\n\n\
                 ### Line Limit Configuration\n\n\
                 Retrieved from config_manager:\n\
                 - get_file_write_line_limit()\n\
                 - Typically 500 lines\n\
                 - Warning generated if exceeded\n\
                 - Edit still proceeds (performance consideration)\n\n\
                 ### Error Type Mapping\n\n\
                 - Empty string: McpError::InvalidArguments\n\
                 - Identical strings: McpError::InvalidArguments\n\
                 - File not found: McpError from fs::read_to_string\n\
                 - Permission denied: McpError from fs::write\n\
                 - No match (fuzzy failed): McpError::InvalidArguments\n\
                 - Count mismatch: NOT an error (success with warning)"
            );
        }

        content
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for EditBlockTool {
    type Args = FsEditBlockArgs;
    type PromptArgs = FsEditBlockPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::filesystem::FS_EDIT_BLOCK
    }

    fn description() -> &'static str {
        "Apply surgical text replacements to files. Takes old_string and new_string, and performs \
         exact string replacement. By default replaces one occurrence. To replace multiple, set \
         expected_replacements. Returns error if old_string not found, or warning if actual count \
         doesn't match expected. Automatically validates paths."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true // Modifies file content
    }

    fn idempotent() -> bool {
        false // Each replacement changes content
    }

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let start_time = Instant::now(); // START TIMER

        // Validate inputs
        if args.old_string.is_empty() {
            return Err(McpError::InvalidArguments(
                "Empty search strings are not allowed. Please provide a non-empty string to search for.".to_string()
            ));
        }

        if args.old_string == args.new_string {
            return Err(McpError::InvalidArguments(
                "old_string and new_string are identical. No changes would be made.".to_string(),
            ));
        }

        let valid_path = validate_path(&args.path, &self.config_manager).await?;

        // Get file extension for response
        let extension = valid_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_string();

        // Read file
        let content = fs::read_to_string(&valid_path).await?;

        // Detect file's line ending style
        let file_line_ending = detect_line_ending(&content);

        // Normalize search string to match file's line endings
        let normalized_old_string = normalize_line_endings(&args.old_string, file_line_ending);

        // Check line limits and generate warning if exceeded
        let line_limit = self.config_manager.get_file_write_line_limit();
        let search_lines = normalized_old_string.lines().count().max(1);
        let replace_lines = args.new_string.lines().count().max(1);
        let max_lines = search_lines.max(replace_lines);

        let warning = if max_lines > line_limit {
            let problem_text = if search_lines > replace_lines {
                "search text"
            } else {
                "replacement text"
            };
            format!(
                "\n\nWARNING: The {problem_text} has {max_lines} lines (maximum: {line_limit}).\n\n\
                 RECOMMENDATION: For large search/replace operations, consider breaking them \
                 into smaller chunks with fewer lines."
            )
        } else {
            String::new()
        };

        // Count occurrences using normalized search
        let occurrence_count = content.matches(&normalized_old_string).count();

        // Handle no exact matches - try fuzzy search
        if occurrence_count == 0 {
            // Measure fuzzy search performance
            let start = std::time::Instant::now();

            // Attempt fuzzy match
            let fuzzy_result = recursive_fuzzy_index_of_with_defaults(&content, &args.old_string);

            // Calculate elapsed time in milliseconds
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Calculate similarity using standard function
            let similarity = get_similarity_ratio(&fuzzy_result.value, &args.old_string);

            // Get configurable threshold from config
            let threshold = self.config_manager.get_fuzzy_search_threshold();

            // Get execution time for logging
            let execution_time = start_time.elapsed().as_secs_f64() * 1000.0;

            // Log fuzzy search attempt (FIRE-AND-FORGET, NEVER BLOCKS!)
            let log_entry = EditBlockLogEntry {
                timestamp: Utc::now(),
                search_text: args.old_string.clone(),
                found_text: Some(fuzzy_result.value.clone()),
                similarity: Some(similarity),
                execution_time_ms: execution_time,
                exact_match_count: 0,
                expected_replacements: args.expected_replacements,
                fuzzy_threshold: threshold,
                below_threshold: similarity < threshold,
                diff: None,
                search_length: args.old_string.len(),
                found_length: Some(fuzzy_result.value.len()),
                file_extension: extension.clone(),
                character_codes: None,
                unique_character_count: None,
                diff_length: None,
                result: if similarity >= threshold {
                    EditBlockResult::FuzzyMatchAccepted
                } else {
                    EditBlockResult::FuzzyMatchRejected
                },
            };

            get_edit_logger().log(log_entry);

            if similarity >= threshold {
                // Found similar text - show character diff
                let diff = CharDiff::new(&args.old_string, &fuzzy_result.value);
                let diff_display = diff.format();
                let is_whitespace_only = diff.is_whitespace_only();

                // Calculate line number where match was found
                let line_number = count_lines_before_index(&content, fuzzy_result.start);

                // Log the fuzzy match attempt
                let logger = get_logger().await;
                let fuzzy_log_entry = FuzzySearchLogEntry {
                    timestamp: Utc::now(),
                    search_text: args.old_string.clone(),
                    found_text: fuzzy_result.value.clone(),
                    similarity,
                    execution_time_ms: elapsed_ms,
                    exact_match_count: 0,
                    expected_replacements: args.expected_replacements,
                    fuzzy_threshold: threshold,
                    below_threshold: false,
                    diff: diff_display.clone(),
                    search_length: args.old_string.len(),
                    found_length: fuzzy_result.value.len(),
                    file_extension: extension.clone(),
                };

                let _ = logger.log(&fuzzy_log_entry).await; // Ignore log errors
                let log_path = Some(logger.log_path().to_path_buf());
                drop(logger); // Release lock

                // Build suggestion context
                let context = SuggestionContext {
                    file_path: args.path.clone(),
                    search_string: args.old_string.clone(),
                    line_number: Some(line_number),
                    log_path,
                    execution_time_ms: Some(elapsed_ms),
                };

                // Build user-facing suggestion
                let suggestion = Suggestion::for_failure(
                    &EditFailureReason::FuzzyMatchAboveThreshold {
                        similarity,
                        is_whitespace_only,
                    },
                    &context,
                );

                // Perform comprehensive character analysis
                let char_data = CharCodeData::analyze(&args.old_string, &fuzzy_result.value);

                // Build complete error message
                let mut error_msg = suggestion.message.clone();
                error_msg.push_str("\n\nCharacter-level differences:\n");
                error_msg.push_str(&diff_display);

                if is_whitespace_only {
                    error_msg.push_str("\n\nNote: Difference is whitespace only.");
                }

                // Add comprehensive character analysis
                error_msg.push_str("\n\n");
                error_msg.push_str(&char_data.format_detailed_report());

                error_msg.push_str(&suggestion.format());

                return Err(McpError::InvalidArguments(error_msg));
            }

            // Calculate line number where match was found
            let line_number = count_lines_before_index(&content, fuzzy_result.start);

            // Log the fuzzy match attempt (below threshold)
            let diff = CharDiff::new(&args.old_string, &fuzzy_result.value);
            let diff_display = diff.format();

            let logger = get_logger().await;
            let fuzzy_log_entry = FuzzySearchLogEntry {
                timestamp: Utc::now(),
                search_text: args.old_string.clone(),
                found_text: fuzzy_result.value.clone(),
                similarity,
                execution_time_ms: elapsed_ms,
                exact_match_count: 0,
                expected_replacements: args.expected_replacements,
                fuzzy_threshold: threshold,
                below_threshold: true,
                diff: diff_display,
                search_length: args.old_string.len(),
                found_length: fuzzy_result.value.len(),
                file_extension: extension.clone(),
            };

            let _ = logger.log(&fuzzy_log_entry).await; // Ignore log errors
            let log_path = Some(logger.log_path().to_path_buf());
            drop(logger); // Release lock

            // No good fuzzy match found - below threshold
            let context = SuggestionContext {
                file_path: args.path.clone(),
                search_string: args.old_string.clone(),
                line_number: Some(line_number),
                log_path,
                execution_time_ms: Some(elapsed_ms),
            };

            let suggestion = Suggestion::for_failure(
                &EditFailureReason::FuzzyMatchBelowThreshold {
                    similarity,
                    threshold,
                    found_text: fuzzy_result.value.clone(),
                },
                &context,
            );

            // Build complete error message with BOTH message and suggestions
            let mut error_msg = String::new();
            error_msg.push_str(&suggestion.message);
            error_msg.push_str(&suggestion.format());

            return Err(McpError::InvalidArguments(error_msg));
        }

        // Perform replacement using normalized strings
        let normalized_new_string = normalize_line_endings(&args.new_string, file_line_ending);
        let new_content = content.replace(&normalized_old_string, &normalized_new_string);

        fs::write(&valid_path, &new_content).await?;

        // Build response based on match status
        let execution_time = start_time.elapsed().as_secs_f64() * 1000.0;

        // Log successful exact match (FIRE-AND-FORGET, NEVER BLOCKS!)
        let log_entry = EditBlockLogEntry {
            timestamp: Utc::now(),
            search_text: args.old_string.clone(),
            found_text: Some(args.old_string.clone()),
            similarity: Some(1.0), // Exact match
            execution_time_ms: execution_time,
            exact_match_count: occurrence_count,
            expected_replacements: args.expected_replacements,
            fuzzy_threshold: self.config_manager.get_fuzzy_search_threshold(),
            below_threshold: false,
            diff: None,
            search_length: args.old_string.len(),
            found_length: Some(args.old_string.len()),
            file_extension: extension.clone(),
            character_codes: None,
            unique_character_count: None,
            diff_length: None,
            result: EditBlockResult::ExactMatch,
        };

        get_edit_logger().log(log_entry);

        if occurrence_count == args.expected_replacements {
            // Exact match - success
            let mut contents = Vec::new();

            // Human summary
            let delta = args.new_string.len() as i64 - args.old_string.len() as i64;
            let delta_str = if delta >= 0 {
                format!("+{}", delta)
            } else {
                format!("{}", delta)
            };
            let display_path = display_path_relative_to_git_root(
                std::path::Path::new(&args.path),
                ctx.git_root()
            );
            let summary = format!(
                "\x1b[33m󰆐 {} replacement(s) in {}\x1b[0m\n\
                 󰢬 Precision: {} → {} bytes (delta: {}){}",
                occurrence_count,
                display_path,
                args.old_string.len(),
                args.new_string.len(),
                delta_str,
                warning
            );
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "path": args.path,
                "replacements": occurrence_count,
                "old_bytes": args.old_string.len(),
                "new_bytes": args.new_string.len(),
                "delta_bytes": delta,
                "expected_replacements": args.expected_replacements,
                "matched_expected": true,
                "file_extension": extension
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));

            Ok(contents)
        } else {
            // Mismatch - success with warning and suggestions
            let context = SuggestionContext {
                file_path: args.path.clone(),
                search_string: args.old_string.clone(),
                line_number: None,
                log_path: None,
                execution_time_ms: None,
            };

            let suggestion = Suggestion::for_failure(
                &EditFailureReason::UnexpectedCount {
                    expected: args.expected_replacements,
                    found: occurrence_count,
                },
                &context,
            );

            // Return success with warning and suggestions
            let mut contents = Vec::new();

            // Human summary with warning
            let delta = args.new_string.len() as i64 - args.old_string.len() as i64;
            let delta_str = if delta >= 0 {
                format!("+{}", delta)
            } else {
                format!("{}", delta)
            };
            let display_path = display_path_relative_to_git_root(
                std::path::Path::new(&args.path),
                ctx.git_root()
            );
            let summary = format!(
                "\x1b[33m󰆐 {} replacement(s) in {}\x1b[0m\n 󰢬 Precision: {} → {} bytes (delta: {}) · Expected: {} · See Content[1] for details{}",
                occurrence_count,
                display_path,
                args.old_string.len(),
                args.new_string.len(),
                delta_str,
                args.expected_replacements,
                warning
            );
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "path": args.path,
                "replacements": occurrence_count,
                "old_bytes": args.old_string.len(),
                "new_bytes": args.new_string.len(),
                "delta_bytes": delta,
                "expected_replacements": args.expected_replacements,
                "matched_expected": false,
                "warning": suggestion.message,
                "file_extension": extension
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));

            Ok(contents)
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "example_focus".to_string(),
                title: Some("Teaching Focus".to_string()),
                description: Some(
                    "Optional aspect to focus on: 'fuzzy-search' for fuzzy matching behavior, \
                     'expected-replacements' for count verification, 'line-endings' for encoding handling, \
                     or 'edge-cases' for error conditions. Omit for comprehensive coverage.".to_string(),
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "show_advanced".to_string(),
                title: Some("Advanced Topics".to_string()),
                description: Some(
                    "Optional boolean: if true, include advanced topics like fuzzy search configuration, \
                     similarity thresholds, and internal behavior. Default: false.".to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let example_focus = args.example_focus.as_deref().unwrap_or("comprehensive");
        let show_advanced = args.show_advanced.unwrap_or(false);

        // Build comprehensive teaching content based on focus area
        let teaching_content = match example_focus {
            "fuzzy-search" => self.build_fuzzy_search_prompt(show_advanced),
            "expected-replacements" => self.build_expected_replacements_prompt(show_advanced),
            "line-endings" => self.build_line_endings_prompt(show_advanced),
            "edge-cases" => self.build_edge_cases_prompt(show_advanced),
            _ => self.build_comprehensive_prompt(show_advanced),
        };

        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use edit_block to make precise changes to files?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(teaching_content),
            },
        ])
    }
}
