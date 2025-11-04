# Race Condition in Timeout Monitoring Task

## Location
`src/search/manager/core.rs:280-309`

## Severity
Low (Error handling issue, not correctness)

## Issue Description
The timeout monitoring task has a check-then-act race condition:

```rust
tokio::spawn(async move {
    let timeout_result = tokio::time::timeout(timeout, search_handle).await;

    match timeout_result {
        Ok(_) => {
            // Search completed - nothing to do
        }
        Err(_elapsed) => {
            // Timeout occurred - send cancellation
            log::warn!("Search session {session_id} timed out");

            // RACE WINDOW: Session could be removed here
            let sessions_guard = sessions.read().await;  // Line 293

            if let Some(session) = sessions_guard.get(&session_id) {
                // Try to send cancellation
                let _ = session.cancellation_tx.send(true);

                // RACE: try_write could fail if cleanup runs here
                if let Ok(mut incomplete) = session.was_incomplete.try_write() {
                    *incomplete = true;  // Line 300
                }
            } else {
                log::debug!("Timeout fired but session {session_id} already cleaned up");
            }
        }
    }
});
```

**The Race:**
1. Search completes at T=59.999s
2. Timeout fires at T=60.000s
3. Cleanup task runs at T=60.001s, removes session
4. Timeout handler acquires read lock at T=60.002s
5. Session already gone → benign log message

**BUT:**
If search completes and cleanup hasn't run yet:
1. Timeout fires
2. Gets read lock on sessions
3. Session still exists
4. Tries to mark incomplete=true
5. Search was actually complete!

## Real-World Impact

### Scenario 1: False "Incomplete" Flag
```
Timeline:
0.000s: Search starts with 60s timeout
59.995s: Search completes naturally (is_complete=true)
60.000s: Timeout monitoring fires
60.001s: Timeout handler marks was_incomplete=true
Result: Session shows both complete=true AND was_incomplete=true (contradictory)
```

This is **confusing for users**:
- "Search completed successfully"
- "But it was also incomplete due to timeout"
- Which is it?

### Scenario 2: try_write Failure
```
Timeline:
60.000s: Timeout fires
60.000s: Another thread reading was_incomplete (holds read lock)
60.001s: Timeout handler calls try_write()
Result: try_write() returns Err, incomplete flag NOT set
```

This is **silent failure**:
- Search actually timed out
- But was_incomplete stays false
- Users don't know search was cut short

## Root Cause
Check-then-act pattern without atomic state transition:
1. Check if session exists (read lock)
2. Check is_complete flag
3. Act on stale information

## Recommended Fix: Check Complete Flag First

```rust
Err(_elapsed) => {
    log::warn!("Search session {session_id} timed out");

    let sessions_guard = sessions.read().await;

    if let Some(session) = sessions_guard.get(&session_id) {
        // CHECK: Is search already complete?
        if session.is_complete.load(Ordering::Acquire) {
            // Search completed naturally, ignore timeout
            log::debug!(
                "Search session {session_id} completed before timeout fired, ignoring"
            );
            return;
        }

        // Search is NOT complete, timeout is real
        let _ = session.cancellation_tx.send(true);

        // Use blocking_write instead of try_write for reliability
        *session.was_incomplete.blocking_write() = true;

        log::info!("Search session {session_id} cancelled due to timeout");
    } else {
        log::debug!(
            "Timeout fired but session {session_id} already cleaned up"
        );
    }
}
```

**Key improvements:**
1. **Check is_complete first:** Avoids marking completed searches as incomplete
2. **blocking_write:** Ensures flag is set (vs try_write silent failure)
3. **Better logging:** Distinguishes "completed before timeout" from "cleaned up"

## Alternative Fix: Atomic Flag Transition

Use a separate atomic state machine:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchState {
    Running = 0,
    CompletedNaturally = 1,
    CompletedByTimeout = 2,
    CompletedByCancellation = 3,
}

pub struct SearchSession {
    // ...
    state: Arc<AtomicU8>,  // SearchState
}

// In search completion:
self.state.compare_exchange(
    SearchState::Running,
    SearchState::CompletedNaturally,
    Ordering::AcqRel,
    Ordering::Acquire,
);

// In timeout handler:
match self.state.compare_exchange(
    SearchState::Running,
    SearchState::CompletedByTimeout,
    Ordering::AcqRel,
    Ordering::Acquire,
) {
    Ok(_) => {
        // Successfully transitioned to timeout state
        send_cancellation();
    }
    Err(_) => {
        // Already completed, ignore timeout
    }
}
```

**Benefits:**
- Atomic state transitions (no races)
- Explicit state machine (clearer semantics)
- Distinguishes completion reasons

**Drawbacks:**
- More complex
- Requires refactoring existing code

## Impact Assessment

| Issue | Likelihood | User Impact | Current | With Fix |
|-------|-----------|-------------|---------|----------|
| False incomplete flag | Medium | Medium | Yes | No |
| Silent try_write failure | Low | Medium | Yes | No |
| Confusing logs | Medium | Low | Yes | No |

## Testing Recommendation

```rust
#[tokio::test]
async fn test_timeout_race_with_natural_completion() {
    // Start search with 100ms timeout
    let session_id = start_search_with_timeout(100).await;

    // Complete search at 99ms (just before timeout)
    tokio::time::sleep(Duration::from_millis(99)).await;
    complete_search_naturally(&session_id).await;

    // Wait for timeout to fire (101ms total)
    tokio::time::sleep(Duration::from_millis(2)).await;

    // Verify: should be complete but NOT incomplete
    let session = get_session(&session_id).await;
    assert!(session.is_complete.load(Ordering::Acquire));
    assert!(!*session.was_incomplete.read().await);  // Should be false
}

#[tokio::test]
async fn test_timeout_race_with_concurrent_read() {
    // Start search with 100ms timeout
    let session_id = start_search_with_timeout(100).await;

    // Hold read lock on was_incomplete during timeout
    let session = get_session(&session_id).await;
    let _guard = session.was_incomplete.read().await;

    // Wait for timeout to fire
    tokio::time::sleep(Duration::from_millis(101)).await;

    // Drop guard, check if incomplete was set
    drop(_guard);

    // Verify: should be marked incomplete despite concurrent read
    assert!(*session.was_incomplete.read().await);
}
```

## Related Issue
Similar pattern exists in `terminate_search()` (line 324-344) but less severe since it's user-initiated.
