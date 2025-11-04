# Infrequent Cancellation Checks Delay Search Termination

## Location
- `src/search/manager/content_search.rs:356-360`
- `src/search/manager/file_search.rs:332-336`

## Severity
Low (User experience issue, not correctness)

## Issue Description
The search code only checks for cancellation every 100 results, which can delay search termination when processing large files or expensive matches:

```rust
// In ContentSearchVisitor::visit()
for (i, result) in results.into_iter().enumerate() {
    // Check cancellation every 100 results to balance responsiveness vs overhead
    if i % 100 == 0 && *self.cancellation_rx.borrow() {  // Line 357
        *self.was_incomplete.blocking_write() = true;
        return ignore::WalkState::Quit;
    }
    // ... process result ...
}
```

**The Problem:**
1. Large file with 10,000 matches → checked 100 times
2. Between checks: processing 99 matches × ~100μs each = **~10ms**
3. User cancels during this 10ms window
4. Cancellation not detected until next check → **additional delay**

## Real-World Impact

### Scenario 1: Minified JavaScript File
```
File: bundle.min.js (1 line, 5MB)
Pattern: "function"
Matches: 15,000 occurrences in single line
```

Processing timeline:
- Parse 100 matches: ~500ms (slow due to huge line)
- User cancels at T=250ms
- Next check at T=500ms
- **Wasted time: 250ms** processing after cancel

### Scenario 2: Large Log File
```
File: application.log (500,000 lines)
Pattern: "ERROR"
Matches: 50,000 errors
```

Processing timeline:
- 100 matches every ~50ms (fast disk I/O)
- User cancels during file processing
- Average delay: ~25ms (half of check interval)
- User perception: "Cancel is sluggish"

### Scenario 3: Regex on Binary-ish Files
```
File: node_modules/package/dist/vendor.js
Pattern: complex regex with backtracking
Matches: 1000, but each takes 10ms to process (regex is expensive)
```

Processing timeline:
- 100 matches: 1000ms (1 second!)
- User cancels at T=500ms
- Next check at T=1000ms
- **Wasted time: 500ms** of CPU-intensive regex

## Why Every 100?

Original reasoning likely:
- **Checking is cheap** (~50ns)
- **But not free:** Branch prediction, atomic load
- **Balance:** Responsiveness vs overhead

Checking every match:
- 10,000 matches × 50ns = 0.5ms overhead (negligible)
- But cache effects, branch misprediction could be worse

Checking every 100:
- 100 checks × 50ns = 5μs overhead (irrelevant)
- Worst-case delay: 100 × avg_match_time

## Root Cause
Fixed interval chosen for typical case, doesn't adapt to:
1. Match processing time (some are slow)
2. File characteristics (huge files, slow regex)
3. User expectations (cancel should be instant)

## Recommended Fix Option 1: Adaptive Checking

Check more frequently when processing is slow:

```rust
for (i, result) in results.into_iter().enumerate() {
    // Adaptive: check every 10 results (10x more frequent)
    // Overhead: 10x checks = 500ns per 10 matches (still negligible)
    if i % 10 == 0 && *self.cancellation_rx.borrow() {
        *self.was_incomplete.blocking_write() = true;
        return ignore::WalkState::Quit;
    }

    // ... process result ...
}
```

**Benefits:**
- 10x faster cancellation response
- Still negligible overhead (<1μs per 10 matches)
- Simple one-line change

**Measurement:**
- Current: avg delay ~50 matches × avg_time
- Improved: avg delay ~5 matches × avg_time
- **10x improvement in responsiveness**

## Recommended Fix Option 2: Time-Based Checking

Check based on elapsed time instead of count:

```rust
struct ContentSearchVisitor {
    // ...
    last_cancel_check: Instant,
}

for result in results {
    // Check cancellation every 100ms (time-based)
    let now = Instant::now();
    if now.duration_since(self.last_cancel_check) >= Duration::from_millis(100) {
        self.last_cancel_check = now;

        if *self.cancellation_rx.borrow() {
            *self.was_incomplete.blocking_write() = true;
            return ignore::WalkState::Quit;
        }
    }

    // ... process result ...
}
```

**Benefits:**
- Guarantees max 100ms delay (user-facing SLA)
- Adapts automatically to processing speed
- More checks on slow operations
- Fewer checks on fast operations

**Drawbacks:**
- Requires `Instant::now()` call (costs ~20ns vs 0ns for counter)
- More complex logic

## Recommended Fix Option 3: Hybrid Approach

Best of both worlds:

```rust
for (i, result) in results.into_iter().enumerate() {
    // Check every 10 results OR every 100ms, whichever comes first
    let should_check = i % 10 == 0  // Counter-based (cheap)
        || self.start_time.elapsed() >= Duration::from_millis(100);  // Time-based (fallback)

    if should_check && *self.cancellation_rx.borrow() {
        *self.was_incomplete.blocking_write() = true;
        return ignore::WalkState::Quit;
    }

    // ... process result ...
}
```

**Benefits:**
- Fast path: counter check (zero overhead)
- Slow path: time guarantee (max 100ms delay)
- Best responsiveness for all cases

## Performance Analysis

| Approach | Checks per 1000 matches | Overhead | Max Delay | Responsiveness |
|----------|-------------------------|----------|-----------|----------------|
| Current (every 100) | 10 | 0.5μs | Variable | Poor |
| Every 10 | 100 | 5μs | 10x better | Good |
| Time-based (100ms) | Variable | ~20ns per check | 100ms | Excellent |
| Hybrid | 100 (typical) | ~5-20μs | 100ms | Excellent |

## User Experience Impact

### Current Behavior
```
User: *clicks cancel*
System: *processes 50 more matches* (300ms)
System: Finally quits
User: "Why is cancel so slow?!"
```

### With Fix (Every 10)
```
User: *clicks cancel*
System: *processes 5 more matches* (30ms)
System: Quits quickly
User: "Good responsiveness"
```

### With Fix (Time-based)
```
User: *clicks cancel*
System: *processes for up to 100ms*
System: Quits within SLA
User: "Instant cancel"
```

## Similar Issues in File Search

File search has similar pattern (line 332-336):
```rust
// Check for cancellation
if *self.cancellation_rx.borrow() {
    *self.was_incomplete.blocking_write() = true;
    return ignore::WalkState::Quit;
}
```

But this checks EVERY file, which is appropriate because:
- Files are the primary unit of iteration
- Checking per-file is already frequent
- No matches loop to check within

**Recommendation for file search:** Keep as-is (already optimal).

## Recommendation

**Implement Option 1 (every 10) for content search:**
1. Simple change: `i % 100 == 0` → `i % 10 == 0`
2. Significant improvement in responsiveness
3. Negligible overhead
4. Easy to adjust if needed (could try every 5 or 20)

**If users still report sluggishness:**
- Upgrade to Option 3 (hybrid) with time guarantee

## Testing Recommendation

### Unit Test: Cancellation Responsiveness
```rust
#[test]
fn test_cancellation_response_time() {
    let (tx, rx) = watch::channel(false);

    // Create visitor with 10,000 pending results
    let mut visitor = create_visitor_with_results(10_000, rx);

    // Start processing in thread
    let handle = thread::spawn(move || {
        let start = Instant::now();
        visitor.process_all_results();
        start.elapsed()
    });

    // Cancel after 100ms
    thread::sleep(Duration::from_millis(100));
    tx.send(true).unwrap();

    // Measure how long it takes to actually stop
    let elapsed = handle.join().unwrap();

    // With current code: could be up to 500ms
    // With fix: should be <150ms (100ms + 1 check interval)
    assert!(
        elapsed < Duration::from_millis(150),
        "Cancellation took too long: {}ms",
        elapsed.as_millis()
    );
}
```

### Integration Test: User-Facing Latency
```rust
#[tokio::test]
async fn test_cancel_feels_instant() {
    // Start large search
    let session_id = start_large_search().await;

    // Wait for it to be running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel and measure response time
    let cancel_start = Instant::now();
    terminate_search(&session_id).await.unwrap();

    // Check session status
    loop {
        let session = get_session(&session_id).await;
        if session.is_complete.load(Ordering::Acquire) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let cancel_latency = cancel_start.elapsed();

    // Should feel instant (< 200ms)
    assert!(
        cancel_latency < Duration::from_millis(200),
        "Cancel latency too high: {}ms",
        cancel_latency.as_millis()
    );
}
```

## Monitoring

Add metric for cancellation latency:
```rust
// When cancellation detected
let cancel_detected = Instant::now();
let latency_ms = cancel_detected
    .duration_since(self.cancel_requested_time)
    .as_millis();

log::info!("Cancellation latency: {}ms (processed {} more matches)", latency_ms, i);
```

This helps track real-world responsiveness and tune the check frequency.
