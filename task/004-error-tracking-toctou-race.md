# Error Tracking Can Exceed MAX_DETAILED_ERRORS Limit

## Location
- `src/search/manager/content_search.rs:154-185`
- `src/search/manager/file_search.rs:218-247`

## Severity
Low (Minor memory issue, not critical)

## Issue Description
The error tracking code has a TOCTOU (Time-Of-Check-Time-Of-Use) race condition that can allow the error list to exceed the intended `MAX_DETAILED_ERRORS` limit:

```rust
fn track_error(&self, error: &ignore::Error) {
    // Atomic increment - correct
    self.error_count.fetch_add(1, Ordering::SeqCst);

    // TOCTOU RACE WINDOW STARTS HERE
    let should_store = {
        let errors = self.errors.blocking_read();  // Check: read lock
        errors.len() < MAX_DETAILED_ERRORS         // Check: len < 100
    };  // Read lock released

    // OTHER THREADS CAN INSERT HERE

    if should_store {
        let error_str = error.to_string();
        // Allocate error data...

        let mut errors = self.errors.blocking_write();  // Use: write lock

        // Double-check (mitigation but not perfect)
        if errors.len() < MAX_DETAILED_ERRORS {
            errors.push(SearchError { /* ... */ });    // Use: insert
        }
    }
}
```

**The Race:**
1. Thread A checks: `len=99 < 100` → should_store=true
2. Thread B checks: `len=99 < 100` → should_store=true
3. Thread A writes: len=100
4. Thread B writes: len=101 **LIMIT EXCEEDED**

With N threads racing, list can grow to `MAX_DETAILED_ERRORS + N - 1`.

## Real-World Impact
- **Scenario:** 8-thread parallel search, all hitting errors simultaneously
- **Expected limit:** 100 errors
- **Actual limit:** Up to 107 errors (100 + 8 - 1)
- **Memory impact:**
  - Each `SearchError`: ~200 bytes average
  - 7 extra errors: ~1.4 KB extra (negligible)
- **Functional impact:** Minimal - error limit is soft guidance, not hard requirement

**Why low severity:**
- Extra allocations are bounded (max +N-1 where N = thread count)
- Doesn't affect correctness
- Memory overhead is small (~1-2 KB worst case)
- Only occurs during error-heavy searches

## Root Cause
Two-phase locking pattern:
1. Read lock to check capacity
2. Release lock
3. Write lock to insert

The gap between (1) and (3) allows races.

## Recommended Fix Option 1: Atomic Counter
Use atomic counter instead of vec length:

```rust
struct ErrorTracking {
    count: Arc<AtomicUsize>,
    errors: Arc<RwLock<Vec<SearchError>>>,
    stored_count: Arc<AtomicUsize>,  // NEW: tracks stored count
}

fn track_error(&self, error: &ignore::Error) {
    self.count.fetch_add(1, Ordering::SeqCst);

    // Atomic reservation - no TOCTOU
    let slot = self.stored_count.fetch_update(
        Ordering::SeqCst,
        Ordering::SeqCst,
        |current| {
            if current < MAX_DETAILED_ERRORS {
                Some(current + 1)  // Reserve slot atomically
            } else {
                None  // Limit reached
            }
        },
    );

    if let Ok(_reserved_slot) = slot {
        // We have a guaranteed slot, safe to allocate and insert
        let error_str = error.to_string();
        let error_data = SearchError { /* ... */ };

        let mut errors = self.errors.blocking_write();
        errors.push(error_data);  // Guaranteed to not exceed limit
    }
}
```

**Benefits:**
- Atomic reservation prevents races
- Guarantees exactly MAX_DETAILED_ERRORS limit
- No TOCTOU window

## Recommended Fix Option 2: Accept the Race (Document It)
Given the low severity, document the race as acceptable:

```rust
/// Track a directory traversal error
///
/// CONCURRENCY NOTE: Due to check-then-insert pattern, the error list may
/// slightly exceed MAX_DETAILED_ERRORS during heavy concurrent error scenarios
/// (up to MAX_DETAILED_ERRORS + thread_count - 1). This is acceptable as the
/// limit is soft guidance to prevent memory bloat, not a hard requirement.
fn track_error(&self, error: &ignore::Error) {
    // ...existing code...
}
```

**Benefits:**
- Zero code changes
- Performance unchanged
- Acknowledges intentional trade-off

## Recommendation
**Option 2 (document)** is preferred because:
- Memory impact is negligible (< 2 KB worst case)
- Fix adds complexity for minimal benefit
- Current double-check already provides reasonable mitigation
- Errors only stored during actual error conditions

## Testing Recommendation
Add concurrent test to verify bounded behavior:
```rust
#[test]
fn test_error_tracking_bounded_under_contention() {
    // Spawn 16 threads simultaneously adding errors
    // Verify final error count is bounded (≤ MAX_DETAILED_ERRORS + 16)
}
```
