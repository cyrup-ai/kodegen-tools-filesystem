//! Integration tests for ripgrep flags
//! This file includes all flag-related tests from the search/rg/flags hierarchy

#[path = "search/rg/flags/defs/output/test_separator_flags.rs"]
mod test_separator_flags;

#[path = "search/rg/flags/defs/output/test_limit_flags.rs"]
mod test_limit_flags;

#[path = "search/rg/flags/defs/output/test_context_flags.rs"]
mod test_context_flags;

#[path = "search/rg/flags/defs/output/test_color_flags.rs"]
mod test_color_flags;

#[path = "search/rg/flags/defs/output/test_output_mode_flags.rs"]
mod test_output_mode_flags;

#[path = "search/rg/flags/defs/output/test_display_flags.rs"]
mod test_display_flags;

#[path = "search/rg/flags/defs/test_other_behaviors.rs"]
mod test_other_behaviors;

#[path = "search/rg/flags/defs/test_output_modes.rs"]
mod test_output_modes;
