# Code Review Task List - src/ Module

This directory contains detailed task files for issues identified during code review of the `src/` module.

## Summary

**Total Issues Found:** 13

### By Severity
- **High:** 1 issue (hot path performance)
- **Medium:** 8 issues (race conditions, performance, error handling)
- **Low:** 4 issues (minor optimizations, UX improvements)

### By Category
- **Race Conditions:** 3 issues
- **Performance:** 5 issues
- **Error Handling:** 3 issues
- **Code Quality:** 2 issues

## Issues by Priority

### High Priority (Fix Soon)
1. **003-hot-path-string-allocation.md** - Unnecessary allocations in file search (20-100ms saved)
2. **007-atomic-ordering-performance.md** - Over-use of SeqCst hurts ARM performance (15-25% improvement)
3. **012-lock-contention-during-sort.md** - Write lock held during expensive sort (80ms → <1ms)

### Medium Priority (Fix When Convenient)
4. **001-race-condition-uuid-collision.md** - TOCTOU in UUID collision detection (low probability)
5. **002-recursive-boxed-futures-stack-risk.md** - Boxed async recursion (deep paths issue)
6. **005-path-normalization-overhead.md** - Always lowercasing paths (40-50% improvement)
7. **006-drop-panic-safety.md** - Drop can cause double panic (process abort risk)
8. **008-timeout-monitoring-race.md** - Timeout handler race with completion
9. **009-canonicalize-silent-failure.md** - Silent symlink resolution failures
10. **010-preprocessor-contract-violation.md** - Runtime error for programming bug

### Low Priority (Nice to Have)
11. **004-error-tracking-toctou-race.md** - Error list can slightly exceed limit (negligible impact)
12. **011-json-buffer-allocation.md** - Small JSON buffer causes reallocations
13. **013-missing-cancellation-checks.md** - Infrequent cancellation checks (UX issue)

## Quick Reference

| File | Issue | Impact | Fix Complexity | Est. Time |
|------|-------|--------|----------------|-----------|
| 003 | Hot path allocations | High | Low | 30 min |
| 007 | Atomic ordering | Medium-High | Medium | 2 hours |
| 012 | Lock contention | High | Medium | 1 hour |
| 001 | UUID race | Low | Low | 30 min |
| 002 | Recursive futures | Medium | Low | 30 min |
| 005 | Path normalization | Medium | Medium | 1 hour |
| 006 | Drop panic safety | Medium | Medium | 1 hour |
| 008 | Timeout race | Low | Low | 30 min |
| 009 | Silent failures | Low | Low | 30 min |
| 010 | Preprocessor contract | Low | Low | 15 min |
| 004 | Error tracking race | Very Low | N/A | Document only |
| 011 | Buffer size | Low | Trivial | 5 min |
| 013 | Cancellation checks | Low | Trivial | 5 min |

## Key Findings

### Performance Issues
The most impactful performance issues are:
1. **File search string allocations** (003): Unnecessary `to_lowercase()` calls on every file
2. **Lock contention during sort** (012): Holds write lock for 80ms+ on large result sets
3. **Atomic ordering overhead** (007): Over-use of SeqCst on ARM architectures
4. **Path normalization** (005): Always normalizing even when not needed

**Combined potential improvement:** 20-35% faster searches on typical workloads

### Race Conditions
All identified races have low probability or minor impact:
1. **UUID collision** (001): ~1 in 2^122 probability
2. **Error tracking overflow** (004): May exceed limit by thread count
3. **Timeout monitoring** (008): False incomplete flag on completion edge case

**None are critical**, but good practice dictates fixing race conditions regardless of probability.

### Error Handling
Several error handling improvements identified:
1. **Silent symlink failures** (009): Should log warnings for broken/circular symlinks
2. **Contract violations** (010): Should use assertions, not runtime errors
3. **Double panic risk** (006): Drop implementations should use catch_unwind

## Testing Recommendations

Each task file includes specific testing recommendations. Key test additions needed:
- Concurrent stress tests for race conditions
- Benchmarks for performance improvements
- Deep path tests for recursion limits
- Cancellation latency tests for UX

## Architecture Notes

The codebase is well-structured with:
- Clear separation between file and content search
- Good use of parallel iteration with `ignore` crate
- Proper async/await patterns
- Reasonable use of atomics and locks

**Main areas for improvement:**
- Reduce lock hold times (especially during sorting)
- Optimize hot paths (file iteration, string operations)
- Add more defensive error handling
- Consider memory pooling for frequently allocated buffers

## Non-Issues (By Design)

These were examined but are intentional trade-offs:
- No test coverage issues (explicitly excluded from review)
- Thread-local buffers for results (intentional batching optimization)
- Session cleanup retention times (reasonable defaults, configurable)
- Use of blocking operations in blocking thread pool (appropriate for ripgrep)

## Excluded from Review

As requested, this review did NOT focus on:
- Test coverage
- Benchmark coverage
- Documentation completeness
- API design choices

Focus was exclusively on:
- Runtime performance
- Code clarity
- Hidden errors
- Real-world production issues

## Next Steps

Recommended implementation order:
1. Quick wins: 011 (buffer size), 013 (cancellation), 010 (assertions) - 30 minutes total
2. Hot path optimization: 003 (string alloc) - 30 minutes
3. Lock contention: 012 (sort lock) - 1 hour
4. Atomic ordering: 007 (SeqCst) - 2 hours
5. Path optimization: 005 (normalization) - 1 hour
6. Safety improvements: 006 (drop panic) - 1 hour
7. Race fixes: 001 (UUID), 008 (timeout) - 1 hour
8. Error handling: 009 (symlinks), 002 (recursion) - 1 hour

**Total estimated effort:** 8-10 hours for all issues

## Contact

For questions or clarifications about any issue, refer to the detailed task file which includes:
- Complete code analysis
- Real-world impact scenarios
- Specific fix recommendations
- Testing strategies
- Performance estimates
