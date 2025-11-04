# Unnecessary String Allocations in File Search Hot Path

## Location
`src/search/manager/file_search.rs:372-384`

## Severity
High (Performance bottleneck in production)

## Issue Description
The file search loop unconditionally allocates a lowercase copy of every filename, even when case-sensitive matching is used:

```rust
fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
    // ...
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // ALWAYS allocates, even for case-sensitive searches
    let file_name_lower = file_name.to_lowercase();  // Line 372

    match self.case_mode {
        CaseMode::Insensitive => file_name_lower.contains(&self.pattern_lower),
        CaseMode::Smart => {
            if self.is_pattern_lowercase {
                file_name_lower.contains(&self.pattern_lower)  // Uses it
            } else {
                file_name.contains(&self.pattern)  // Doesn't use it!
            }
        }
        CaseMode::Sensitive => file_name.contains(&self.pattern),  // Doesn't use it!
    }
}
```

**Wasted Work:**
- **CaseMode::Sensitive:** Always allocates `file_name_lower`, never uses it
- **CaseMode::Smart + uppercase pattern:** Allocates but uses original string
- **Performance cost:** Memory allocation + UTF-8 traversal + lowercase conversion per file

## Real-World Impact
For a typical project search:
- **Files checked:** 10,000 - 100,000+ files
- **Wasted allocations:**
  - Sensitive mode: 100% wasted (all files)
  - Smart mode with uppercase: ~50% wasted (mixed case patterns)
- **Cost per allocation:**
  - Heap allocation: ~10-20ns
  - Lowercase conversion: ~5-10ns per char
  - Average filename: 20-30 chars
  - **Total:** ~300-500ns per file wasted

### Example Scenario
Searching 50,000 files with case-sensitive pattern:
- Wasted time: 50,000 × 400ns = **20ms pure waste**
- Wasted memory: 50,000 × ~30 bytes = **1.5 MB temporary allocations**
- Cache pressure: Thrashing L1/L2 cache with unnecessary data

Searching node_modules (250,000 files):
- Wasted time: 250,000 × 400ns = **100ms pure waste**
- Wasted memory: **7.5 MB temporary allocations**

## Root Cause
The allocation happens unconditionally outside the case mode branch, before checking if it's needed.

## Recommended Fix
Move allocation inside the branch that needs it:

```rust
fn visit(&mut self, entry: Result<DirEntry, ignore::Error>) -> ignore::WalkState {
    // ...
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Branch-first, allocate-on-demand pattern
    let matches = if let Some(ref glob) = self.glob_pattern {
        glob.is_match(file_name)
    } else if self.word_boundary {
        Self::matches_with_word_boundary(
            file_name,
            &self.pattern,
            &self.pattern_lower,  // Already computed once
            self.case_mode,
            self.is_pattern_lowercase,
        )
    } else {
        // Only allocate when needed for case-insensitive matching
        match self.case_mode {
            CaseMode::Insensitive => {
                let file_name_lower = file_name.to_lowercase();
                file_name_lower.contains(&self.pattern_lower)
            }
            CaseMode::Smart => {
                if self.is_pattern_lowercase {
                    let file_name_lower = file_name.to_lowercase();
                    file_name_lower.contains(&self.pattern_lower)
                } else {
                    file_name.contains(&self.pattern)  // Zero allocations
                }
            }
            CaseMode::Sensitive => {
                file_name.contains(&self.pattern)  // Zero allocations
            }
        }
    };
    // ...
}
```

## Performance Improvement Estimate
- **Case-sensitive searches:** 20-100ms saved on large codebases
- **Smart case with uppercase:** 10-50ms saved
- **Memory:** 1-10 MB temporary allocation pressure removed
- **Cache efficiency:** Reduced L1/L2 cache thrashing

## Similar Issue
The same pattern exists in the word boundary matching code (line 114-124) in `matches_with_word_boundary`. Consider applying the same optimization there.

## Testing Recommendation
Add benchmark comparing:
```rust
#[bench]
fn bench_file_search_case_sensitive(b: &mut Bencher) {
    // Search 10,000 files with case-sensitive pattern
    // Measure allocations and time
}

#[bench]
fn bench_file_search_case_insensitive(b: &mut Bencher) {
    // Search 10,000 files with case-insensitive pattern
    // Should show minimal regression from the fix
}
```
