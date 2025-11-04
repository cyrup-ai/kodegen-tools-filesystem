# Lock Held During Expensive Sort Operation Blocks Other Threads

## Location
`src/search/manager/core.rs:169-195`

## Severity
Medium (Performance issue under contention)

## Issue Description
When sorting is enabled, the code holds a write lock on results during the entire sort operation, which can be expensive for large result sets:

```rust
// Apply sorting to results
let sessions = self.sessions.read().await;  // Line 169
let session = sessions.get(&session_id)?;

let mut results = session.results.write().await;  // Line 174: ACQUIRE WRITE LOCK

if let Some(sort_criterion) = sort_by {
    // ... 10 lines of conversion logic ...

    sort_results(&mut results, sort_by_criterion, sort_dir);  // Line 194: EXPENSIVE SORT
}  // Line 195: Write lock released
```

**The Problem:**
While sorting large result sets (10,000+ items), the write lock is held, blocking:
1. `get_more_results()` calls (can't read results)
2. Background search threads (can't append new results)
3. Other API calls on the same session

## Real-World Impact

### Scenario: Large Search Results
Search for common term in large codebase:
- Pattern: "function" in JavaScript project
- Results: 50,000 matches
- Sort by: modified time (requires timestamp comparison)

**Sort timing:**
- Comparison cost: ~100ns per comparison
- Comparisons needed: ~50,000 × log(50,000) ≈ 800,000
- Total sort time: 800,000 × 100ns = **80ms**

**During these 80ms:**
- Write lock held on results vector
- `get_more_results()` calls blocked → API timeouts
- Background threads trying to add results → blocked (shouldn't happen post-completion, but still)
- Other requests on same session → blocked

### Timeline Example

```
T=0ms:    Start search (50,000 results)
T=1000ms: Search completes
T=1001ms: Sorting starts, WRITE LOCK ACQUIRED
          ├─ T=1010ms: Client calls get_more_results() → BLOCKED
          ├─ T=1020ms: Client calls list_searches() → BLOCKED
          └─ T=1080ms: WRITE LOCK RELEASED, sort complete
T=1081ms: Blocked operations resume

Result: 70ms of unnecessary blocking
```

### Multi-Client Scenario
10 clients simultaneously searching and sorting:
- Each holds write lock for 80ms
- Other clients blocked during each sort
- Effective throughput: 1 sort every 80ms = **12 operations/sec**
- Without contention: **100+ operations/sec**

## Root Cause
Sort-in-place while holding write lock. The sorting algorithm needs mutable access, but:
1. Sorting is read-mostly (comparisons)
2. Swaps are localized
3. No external threads need write access during sort
4. But write lock prevents all reads too!

## Recommended Fix: Copy-Sort-Swap Pattern

```rust
// Phase 1: Clone results with brief read lock
let results_to_sort = {
    let sessions = self.sessions.read().await;
    let session = sessions.get(&session_id)?;

    // Hold read lock only during clone (fast)
    let results = session.results.read().await;
    results.clone()  // ~1-5ms for 50,000 items
};  // Read lock released - other operations can proceed

// Phase 2: Sort without any locks (slow but parallel-safe)
if let Some(sort_criterion) = sort_by {
    use crate::search::sorting::{
        SortBy as SortCriterion, SortDirection as SortDir, sort_results,
    };

    let sort_by_criterion = match sort_criterion {
        SortBy::Path => SortCriterion::Path,
        SortBy::Modified => SortCriterion::Modified,
        SortBy::Accessed => SortCriterion::Accessed,
        SortBy::Created => SortCriterion::Created,
    };

    let sort_dir = match sort_direction.unwrap_or(SortDirection::Ascending) {
        SortDirection::Ascending => SortDir::Ascending,
        SortDirection::Descending => SortDir::Descending,
    };

    // Sort without holding any locks
    sort_results(&mut results_to_sort, sort_by_criterion, sort_dir);
}

// Phase 3: Swap sorted results with brief write lock
{
    let sessions = self.sessions.read().await;
    let session = sessions.get(&session_id)?;

    // Hold write lock only during swap (instant)
    let mut results = session.results.write().await;
    *results = results_to_sort;  // Single pointer swap
}  // Write lock released
```

**Timing breakdown:**
- Phase 1 (read lock): 1-5ms (clone)
- Phase 2 (no locks): 80ms (sort) ← Other threads can proceed here!
- Phase 3 (write lock): <1ms (swap)

**Benefits:**
- Write lock held: 80ms → <1ms (80x reduction)
- Other operations blocked: 80ms → <1ms
- Throughput: 12 ops/sec → 100+ ops/sec

**Trade-off:**
- Memory: Temporary copy of results (~1-5MB for 50,000 items)
- Clone cost: ~1-5ms (but allows parallelism)

## Alternative: Arc + Sort in Background

For extremely large result sets, consider sorting in background:

```rust
pub async fn start_search(&self, options: SearchSessionOptions) -> Result<...> {
    // ... spawn search task ...

    if sort_by.is_some() {
        // Wait for search completion
        // ... existing waiting logic ...

        // Spawn sort as separate task
        let sessions = Arc::clone(&self.sessions);
        let session_id = session_id.clone();
        tokio::spawn(async move {
            // Sort in background
            if let Err(e) = sort_session_results(&sessions, &session_id, sort_by, sort_direction).await {
                log::error!("Background sort failed: {}", e);
            }
        });

        // Return immediately with unsorted results + sorting_in_progress flag
        return Ok(StartSearchResponse {
            session_id,
            is_complete: true,
            is_error: false,
            results: initial_results,  // Unsorted
            sorting_in_progress: true,  // New field
            // ...
        });
    }
    // ...
}
```

**Pros:**
- Zero blocking for clients
- Results available immediately (albeit unsorted)
- Can check sorting status later

**Cons:**
- More complex state management
- API consumers need to handle sorting_in_progress
- Results might be returned in two stages

## Memory vs Latency Trade-off

| Approach | Lock Time | Memory | Complexity | Best For |
|----------|-----------|--------|------------|----------|
| Current (in-place) | 80ms | 0 extra | Simple | Small results (<1000) |
| Copy-sort-swap | <1ms | 1x copy | Simple | Medium results (<100,000) |
| Background sort | 0ms | 1x copy | Complex | Large results, async-friendly |

## Impact Assessment

### For 50,000 results:
- **Current:** 80ms blocking, 0 extra memory
- **Copy-sort-swap:** <1ms blocking, ~5MB extra memory
- **Background:** 0ms blocking, ~5MB extra memory

### For 1,000,000 results:
- **Current:** 2000ms (2 seconds!) blocking
- **Copy-sort-swap:** <1ms blocking, ~100MB extra memory
- **Background:** 0ms blocking, ~100MB extra memory

## Recommendation

**Implement copy-sort-swap pattern:**
1. Simple to implement (~20 lines changed)
2. Dramatically reduces lock contention
3. Memory cost is acceptable (temporary)
4. Works well for typical result sizes (1,000-100,000)

**Consider background sort for v2:**
- If result sets grow very large (>100,000)
- If memory becomes constrained
- If async API is acceptable

## Testing Recommendation

```rust
#[tokio::test]
async fn test_sort_doesnt_block_other_operations() {
    // Start search that will return 50,000 results
    let session_id = start_large_search().await;

    // Wait for completion
    wait_for_completion(&session_id).await;

    // Trigger sort (with instrumentation)
    let sort_start = Instant::now();

    // In parallel, try to access results
    let access_task = tokio::spawn(async move {
        let access_start = Instant::now();
        let _ = get_more_results(&session_id, 0, 100).await;
        access_start.elapsed()  // Should be fast (<10ms)
    });

    // Wait for sort to complete
    let sort_elapsed = sort_start.elapsed();

    // Check that access wasn't blocked for entire sort duration
    let access_elapsed = access_task.await.unwrap();

    assert!(
        access_elapsed < sort_elapsed / 2,
        "Access was blocked for too long: {}ms during {}ms sort",
        access_elapsed.as_millis(),
        sort_elapsed.as_millis()
    );
}

#[bench]
fn bench_sort_with_lock_contention(b: &mut Bencher) {
    // Benchmark sort with concurrent access attempts
    // Measure both sort time and blocked operation latency
}
```

## Monitoring Recommendation

Add metrics to track lock contention:
```rust
// Track time spent holding write lock
let lock_acquired = Instant::now();
let mut results = session.results.write().await;
let lock_wait_time = lock_acquired.elapsed();

// Sort...

let lock_held_time = lock_acquired.elapsed();
log::info!(
    "Sort stats: wait={}ms, held={}ms, count={}",
    lock_wait_time.as_millis(),
    lock_held_time.as_millis(),
    results.len()
);
```

This helps identify when lock contention becomes a problem in production.
