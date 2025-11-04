# Race Condition in UUID Collision Detection

## Location
`src/search/manager/core.rs:74-97`

## Severity
Medium (unlikely to manifest but incorrect pattern)

## Issue Description
The UUID collision detection code has a Time-Of-Check-Time-Of-Use (TOCTOU) race condition:

```rust
let session_id = loop {
    let id = Uuid::new_v4().to_string();

    let sessions = self.sessions.read().await;   // Line 79: Check
    if !sessions.contains_key(&id) {
        drop(sessions);                           // Line 81: Drop read lock
        break id;                                 // Line 82: Use
    }
    // ...collision handling...
};

// Line 134-137: Insert happens later
self.sessions
    .write()
    .await
    .insert(session_id.clone(), session);
```

**The Race:** Between dropping the read lock (line 81) and acquiring the write lock to insert (line 134), another thread could insert a session with the same ID.

## Real-World Impact
- **Probability:** Extremely low (UUID v4 collisions are ~1 in 2^122)
- **Consequence:** If it occurs, two sessions would have the same ID, causing:
  - Session data corruption/mixing
  - Unpredictable search results
  - Potential data races in session state

## Root Cause
The check-and-insert are not atomic. The code drops the read lock before performing the insert with a write lock.

## Recommended Fix
Use `entry()` API with write lock held throughout:

```rust
let session_id = loop {
    let id = Uuid::new_v4().to_string();

    let mut sessions = self.sessions.write().await;
    if !sessions.contains_key(&id) {
        sessions.insert(id.clone(), session);
        break id;
    }

    collision_count += 1;
    log::error!("UUID v4 collision #{collision_count} detected: {id}");

    if collision_count >= 10 {
        return Err(McpError::Other(anyhow::anyhow!(
            "Unable to generate unique session ID after 10 attempts"
        )));
    }
};
```

This makes the check-and-insert atomic by holding the write lock throughout.

## Alternative Fix
Use `sessions.entry(id).or_insert(...)` pattern which is inherently race-free.

## Testing Recommendation
Add concurrent test that attempts to trigger the race:
- Spawn multiple threads generating sessions simultaneously
- Use reduced UUID space (for testing) to increase collision probability
- Verify no duplicate session IDs exist
