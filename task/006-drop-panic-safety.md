# Drop Implementation Could Cause Double Panic

## Location
- `src/search/manager/content_search.rs:529-537`
- `src/search/manager/file_search.rs:441-448`

## Severity
Medium (Could cause immediate process termination)

## Issue Description
The `Drop` implementation flushes buffered results, but if this panics during stack unwinding, it causes a double panic which immediately aborts the process:

```rust
impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        // CRITICAL: Flush any remaining buffered results
        // Without this, the last batch of results would be lost!
        self.flush_buffer();  // COULD PANIC

        // Ensure final last_read_time update
        self.force_update_last_read_time();  // COULD PANIC
    }
}
```

**The Problem:**
1. Search code panics (e.g., assertion failure, out of bounds)
2. Rust unwinds stack, calls `Drop::drop()`
3. `flush_buffer()` acquires lock
4. Lock acquisition panics (e.g., poisoned lock, panic while locked)
5. **Double panic → immediate abort, no backtrace, no cleanup**

## Real-World Scenarios

### Scenario 1: Poisoned Lock
```rust
// Thread 1: Panics while holding write lock
let mut results = self.results.blocking_write();
panic!("Unexpected error");  // Lock is now poisoned

// Thread 2: Drop is called during unwinding
drop() -> flush_buffer() -> blocking_write()
// PoisonError → SECOND PANIC → ABORT
```

### Scenario 2: Recursive Drop
```rust
// Drop calls flush_buffer
flush_buffer() {
    let mut results = self.results.blocking_write();
    results.extend(self.buffer.drain(..));  // Drain could panic
    // If panic here, drops ContentSearchVisitor again
    // Second Drop call → ABORT
}
```

### Scenario 3: OOM During Drop
```rust
flush_buffer() {
    let mut results = self.results.blocking_write();
    results.extend(self.buffer.drain(..));  // Allocation fails
    // OOM panic during drop → ABORT
}
```

## Why This Matters

**Normal panic:** Unwinding allows:
- Stack traces
- Cleanup handlers (finally-style code)
- Error recovery (catch_unwind)
- Graceful degradation

**Abort:** None of the above
- Process terminates immediately
- No diagnostics
- Partial writes could corrupt data
- HTTP server crashes completely

## Current Code Pattern Analysis

```rust
fn flush_buffer(&mut self) {
    if self.buffer.is_empty() {
        return;  // Safe early return
    }

    let was_empty = self.results.blocking_read().is_empty();  // Could panic (poisoned)

    {
        let mut results_guard = self.results.blocking_write();  // Could panic (poisoned)
        results_guard.extend(self.buffer.drain(..));  // Could panic (OOM, logic)
    }

    if was_empty {
        let _ = self.first_result_tx.send(true);  // Could panic (send error)
    }

    // Update atomic timestamp - probably safe
    let elapsed_micros = self.start_time.elapsed().as_micros() as u64;
    self.last_read_time_atomic.store(elapsed_micros, Ordering::Relaxed);
}
```

**Panic points:**
1. `blocking_read()` on poisoned lock
2. `blocking_write()` on poisoned lock
3. `extend()` allocation failure
4. `send()` on closed channel (shouldn't panic but could)

## Recommended Fix: Panic Guard

```rust
impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        // Wrap in catch_unwind to prevent double panic
        let flush_result = catch_unwind(AssertUnwindSafe(|| {
            self.flush_buffer();
        }));

        if let Err(e) = flush_result {
            // Log panic but don't propagate (would cause double panic)
            // Use eprintln since logging might not work during panic
            eprintln!(
                "WARN: flush_buffer panicked in Drop, \
                 last batch of search results may be lost: {:?}",
                e
            );
        }

        // Same for timestamp update
        let _ = catch_unwind(AssertUnwindSafe(|| {
            self.force_update_last_read_time();
        }));
    }
}
```

**Benefits:**
- Prevents double panic → no abort
- Graceful degradation (lose last batch vs lose entire process)
- Logs the issue for debugging
- Allows normal unwinding to continue

**Trade-offs:**
- Uses `AssertUnwindSafe` (requires careful review)
- Last batch of results may be lost on panic
- Slightly more complex code

## Alternative: Try_lock Pattern

```rust
fn flush_buffer_safe(&mut self) {
    if self.buffer.is_empty() {
        return;
    }

    // Use try_write instead of blocking_write
    // Won't panic on poisoned lock, returns Err instead
    let Ok(mut results_guard) = self.results.try_write() else {
        eprintln!("WARN: Could not acquire lock to flush buffer, results may be lost");
        return;
    };

    // Safe extend with catch_unwind
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        results_guard.extend(self.buffer.drain(..));
    })) {
        eprintln!("WARN: Panic during buffer flush: {:?}", e);
        return;
    }
}

impl Drop for ContentSearchVisitor {
    fn drop(&mut self) {
        self.flush_buffer_safe();
        // Similar safe pattern for timestamp
    }
}
```

## Rust Best Practice
From Rust documentation:
> "Drop implementations should be as simple as possible and avoid operations that could panic. Complex cleanup logic should use catch_unwind."

## Impact Assessment

| Risk | Likelihood | Severity | Current | With Fix |
|------|-----------|----------|---------|----------|
| Double panic abort | Low | Critical | Yes | No |
| Lost results on panic | Low | Medium | N/A | Yes |
| Performance overhead | N/A | None | N/A | Negligible |

## Testing Recommendation

```rust
#[test]
fn test_drop_panic_safety() {
    use std::sync::{Arc, Mutex};

    // Create visitor that will panic in drop
    let visitor = create_test_visitor();

    // Poison the lock
    let results = Arc::clone(&visitor.results);
    let handle = thread::spawn(move || {
        let _guard = results.blocking_write();
        panic!("Poison the lock");
    });
    assert!(handle.join().is_err());

    // Now drop visitor - should not abort
    drop(visitor);  // If not fixed, this would abort the entire test
}
```
