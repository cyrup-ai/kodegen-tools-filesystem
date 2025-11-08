# Test Extraction Inventory

**Date**: 2025-11-08  
**Project**: kodegen-tools-filesystem  
**Task**: Extract all tests from `./src/**/*.rs` to `./tests/` with mirrored directory structure

## Status: IN PROGRESS

## Nextest Configuration
- **Version**: cargo-nextest 0.9.105
- **Location**: /Users/davidmaple/.cargo/bin/cargo-nextest
- **Status**: ✅ INSTALLED

## Files Containing Tests

### Current tests/ directory
- `./tests/fixtures/` - Test fixture data (KEEP)

### Files with `#[cfg(test)]` modules or `#[test]` functions

#### 1. Search Module Tests (Currently in src/search/tests/)
- [x] `src/search/tests/binary_mode_tests.rs` → `tests/search/test_binary_mode.rs` ✅
- [x] `src/search/tests/boundary_tests.rs` → `tests/search/test_boundary.rs` ✅
- [x] `src/search/tests/integration_tests.rs` → `tests/search/test_integration.rs` ✅
- [ ] `src/search/tests/mod.rs` → DELETE (test modules will be in tests/)

#### 2. Inline Test Modules in Source Files
- [x] `src/search/sorting.rs` → `tests/search/test_sorting.rs` ✅
- [ ] `src/search/mod.rs` (has `#[cfg(test)] mod tests;`) → REMOVE reference

#### 3. Ripgrep Flags Tests
- [ ] `src/search/rg/flags/config.rs` → `tests/search/rg/flags/test_config.rs`
- [ ] `src/search/rg/flags/parse.rs` → `tests/search/rg/flags/test_parse.rs`
- [ ] `src/search/rg/flags/hiargs/tests.rs` → `tests/search/rg/flags/hiargs/test_hiargs.rs`

#### 4. Ripgrep Flag Definitions - Logging
- [ ] `src/search/rg/flags/defs/logging.rs` → `tests/search/rg/flags/defs/test_logging.rs`
- [ ] `src/search/rg/flags/defs/mod.rs` → `tests/search/rg/flags/defs/test_mod.rs`
- [ ] `src/search/rg/flags/defs/input.rs` → `tests/search/rg/flags/defs/test_input.rs`
- [x] `src/search/rg/flags/defs/output_modes.rs` → `tests/search/rg/flags/defs/test_output_modes.rs` ✅ (3 tests)
- [x] `src/search/rg/flags/defs/other_behaviors.rs` → `tests/search/rg/flags/defs/test_other_behaviors.rs` ✅ (2 tests)

#### 5. Ripgrep Flag Definitions - Output Submodule
- [ ] `src/search/rg/flags/defs/output/output_mode_flags.rs` → `tests/search/rg/flags/defs/output/test_output_mode_flags.rs`
- [ ] `src/search/rg/flags/defs/output/limit_flags.rs` → `tests/search/rg/flags/defs/output/test_limit_flags.rs`
- [ ] `src/search/rg/flags/defs/output/color_flags.rs` → `tests/search/rg/flags/defs/output/test_color_flags.rs`
- [ ] `src/search/rg/flags/defs/output/display_flags.rs` → `tests/search/rg/flags/defs/output/test_display_flags.rs`
- [ ] `src/search/rg/flags/defs/output/context_flags.rs` → `tests/search/rg/flags/defs/output/test_context_flags.rs`
- [ ] `src/search/rg/flags/defs/output/separator_flags.rs` → `tests/search/rg/flags/defs/output/test_separator_flags.rs`

#### 6. Ripgrep Flag Definitions - Search Submodule
- [ ] `src/search/rg/flags/defs/search/encoding_unicode_flags.rs` → `tests/search/rg/flags/defs/search/test_encoding_unicode_flags.rs`
- [ ] `src/search/rg/flags/defs/search/engine_flags.rs` → `tests/search/rg/flags/defs/search/test_engine_flags.rs`
- [ ] `src/search/rg/flags/defs/search/case_and_pattern_flags.rs` → `tests/search/rg/flags/defs/search/test_case_and_pattern_flags.rs`
- [ ] `src/search/rg/flags/defs/search/multiline_boundary_flags.rs` → `tests/search/rg/flags/defs/search/test_multiline_boundary_flags.rs`
- [ ] `src/search/rg/flags/defs/search/limit_and_performance_flags.rs` → `tests/search/rg/flags/defs/search/test_limit_and_performance_flags.rs`

#### 7. Search Manager Tests
- [ ] `src/search/manager/files_mode/mod.rs` → `tests/search/manager/files_mode/test_mod.rs`
- [ ] `src/search/manager/files_mode/tests.rs` → `tests/search/manager/files_mode/test_files_mode.rs`

### Total Files to Process: 27 files

## Directory Structure to Create

```
tests/
├── fixtures/                                          (EXISTS)
├── search/
│   ├── test_sorting.rs
│   ├── test_binary_mode.rs
│   ├── test_boundary.rs
│   ├── test_integration.rs
│   ├── rg/
│   │   ├── flags/
│   │   │   ├── test_config.rs
│   │   │   ├── test_parse.rs
│   │   │   ├── hiargs/
│   │   │   │   └── test_hiargs.rs
│   │   │   └── defs/
│   │   │       ├── test_logging.rs
│   │   │       ├── test_mod.rs
│   │   │       ├── test_input.rs
│   │   │       ├── test_output_modes.rs
│   │   │       ├── test_other_behaviors.rs
│   │   │       ├── output/
│   │   │       │   ├── test_output_mode_flags.rs
│   │   │       │   ├── test_limit_flags.rs
│   │   │       │   ├── test_color_flags.rs
│   │   │       │   ├── test_display_flags.rs
│   │   │       │   ├── test_context_flags.rs
│   │   │       │   └── test_separator_flags.rs
│   │   │       └── search/
│   │   │           ├── test_encoding_unicode_flags.rs
│   │   │           ├── test_engine_flags.rs
│   │   │           ├── test_case_and_pattern_flags.rs
│   │   │           ├── test_multiline_boundary_flags.rs
│   │   │           └── test_limit_and_performance_flags.rs
│   └── manager/
│       └── files_mode/
│           ├── test_mod.rs
│           └── test_files_mode.rs
```

## Cleanup Actions Required

After extraction:
1. Remove `src/search/tests/` directory entirely
2. Remove `#[cfg(test)] mod tests;` from `src/search/mod.rs`
3. Remove all `#[cfg(test)] mod tests { ... }` blocks from source files
4. Remove `src/search/manager/files_mode/tests.rs` 
5. Remove `src/search/manager/files_mode/tests.rs.backup`
6. Update imports in extracted tests to reference `use kodegen_tools_filesystem::...`

## Verification Steps

1. [ ] All tests compile: `cargo test --no-run`
2. [ ] All tests pass: `cargo nextest run`
3. [ ] No test code remains in src/: `grep -r "#\[test\]" src/`
4. [ ] No cfg(test) modules in src/: `grep -r "#\[cfg(test)\]" src/`

## Notes

- Each extraction requires manual review to ensure proper imports
- Test fixtures in `tests/fixtures/` should be preserved
- Some tests may have dependencies on internal module structure that need adjustment
