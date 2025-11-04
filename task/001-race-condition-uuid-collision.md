# Race Condition in UUID Collision Detection

## Location
[`src/search/manager/core.rs:74-137`](../src/search/manager/core.rs)

## Severity
**Medium** - Unlikely to manifest but incorrect concurrency pattern that violates atomicity guarantees

## Core Objective
Eliminate the Time-Of-Check-Time-Of-Use (TOCTOU) race condition in session ID generation to ensure that session ID uniqueness checks and insertions happen atomically, preventing the theoretical possibility of duplicate session IDs in concurrent scenarios.

---

## Deep Analysis

### The Race Condition Explained

**Current Implementation Flow:**
```rust
// Step 1: Generate ID (line 77)
let id = Uuid::new_v4().to_string();

// Step 2: Check if exists with READ lock (lines 79-82)
let sessions = self.sessions.read().await;   // ← ACQUIRE READ LOCK
if !sessions.contains_key(&id) {
    drop(sessions);                           // ← DROP READ LOCK
    break id;                                 // ← Return ID
}
// [GAP IN ATOMICITY - NO LOCKS HELD]

// Step 3: Build session object (lines 109-131)
let session = SearchSession { /* ... */ };

// Step 4: Insert with WRITE lock (lines 134-137)
self.sessions
    .write()                                  // ← ACQUIRE WRITE LOCK
    .await
    .insert(session_id.clone(), session);     // ← INSERT
```

**The Race Window:**
```
Thread A                          Thread B
─────────────────────────────────────────────────────
Generate ID "abc123"
Check: ID not in map ✓
Drop read lock
                                  Generate ID "abc123" (impossible but theoretical)
                                  Check: ID not in map ✓
                                  Drop read lock
                                  Build session
                                  Insert "abc123" → session_b ✓
Build session
Insert "abc123" → session_a      ← OVERWRITES session_b!
```

**Why This Matters:**
Even though UUID v4 collisions are ~1 in 2^122 (~5.3 × 10^36), the pattern itself is incorrect:
- Check-then-act without atomicity is a race condition regardless of probability
- If it ever occurs, consequences are severe: session data corruption, mixed results, undefined behavior
- Code review tools and static analyzers flag TOCTOU patterns as bugs

### Current Code Structure

**Type Definitions:**
```rust
// From src/search/manager/core.rs:28-31
pub struct SearchManager {
    sessions: Arc<RwLock<HashMap<String, SearchSession>>>,
    config_manager: kodegen_tools_config::ConfigManager,
}
```

**Session Structure (20+ fields):**
[See full definition in src/search/types.rs:153-182](../src/search/types.rs)

Key insight: `SearchSession` construction is **fast** - all fields are either:
- Simple copies (String, bool, enum)
- `Arc` clones (just pointer copies)
- `Arc::new()` wrapping simple values

There is **no I/O, no expensive computation, no blocking** in session construction.

---

## Implementation Solution

### Strategy: Atomic Check-and-Insert with Write Lock

Move the session construction **before** the lock acquisition, then hold the write lock for both the check and insert operations.

**Why this approach:**
1. ✅ **Atomic:** Check and insert happen under same lock with no gap
2. ✅ **Simple:** Minimal code changes, no complex patterns needed
3. ✅ **Fast:** Session construction is cheap (~microseconds), lock held briefly
4. ✅ **Correct:** Eliminates race condition entirely

### Detailed Changes Required

**File:** `src/search/manager/core.rs`

**Function:** `SearchManager::start_search()` (line 47)

**Change Location:** Lines 76-137

#### Step-by-Step Transformation

**BEFORE (Current Code):**
```rust
let session_id = loop {
    let id = Uuid::new_v4().to_string();

    let sessions = self.sessions.read().await;   // Read lock
    if !sessions.contains_key(&id) {
        drop(sessions);                           // Drop lock - RACE GAP
        break id;
    }

    collision_count += 1;
    log::error!("UUID v4 collision #{collision_count} detected: {id}...");

    if collision_count >= 10 {
        return Err(McpError::Other(anyhow::anyhow!("...")));
    }
};

// More code here...
let validated_path = validate_path(&options.root_path, &self.config_manager).await?;
let (cancellation_tx, cancellation_rx) = watch::channel(false);
let (first_result_tx, mut first_result_rx) = watch::channel(false);

// Build session (lines 109-131)
let session = SearchSession {
    id: session_id.clone(),
    // ... 20+ fields ...
};

// Insert session (lines 134-137) - RACE GAP from line 82!
self.sessions
    .write()
    .await
    .insert(session_id.clone(), session);
```

**AFTER (Fixed Code):**
```rust
// Validate path FIRST (no point generating ID if path invalid)
let validated_path = validate_path(&options.root_path, &self.config_manager).await?;

// Create channels BEFORE the loop (reused across collision retries)
let (cancellation_tx, cancellation_rx) = watch::channel(false);
let (first_result_tx, mut first_result_rx) = watch::channel(false);

// Generate ID and atomically check-and-insert
let session_id = loop {
    let id = Uuid::new_v4().to_string();

    // Pre-build session object BEFORE lock (fast, no I/O)
    let session = SearchSession {
        id: id.clone(),
        cancellation_tx: cancellation_tx.clone(),
        first_result_tx: first_result_tx.clone(),
        results: Arc::new(RwLock::new(Vec::new())),
        is_complete: Arc::new(AtomicBool::new(false)),
        is_error: Arc::new(RwLock::new(false)),
        error: Arc::new(RwLock::new(None)),
        total_matches: Arc::new(AtomicUsize::new(0)),
        total_files: Arc::new(AtomicUsize::new(0)),
        last_read_time_atomic: Arc::new(AtomicU64::new(0)),
        start_time: Instant::now(),
        was_incomplete: Arc::new(RwLock::new(false)),
        search_type: options.search_type.clone(),
        pattern: options.pattern.clone(),
        timeout_ms: options.timeout_ms,
        error_count: Arc::new(AtomicUsize::new(0)),
        errors: Arc::new(RwLock::new(Vec::new())),
        max_results: effective_max_results,
        output_mode: options.output_mode,
        seen_files: Arc::new(RwLock::new(std::collections::HashSet::new())),
        file_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
    };

    // Atomic check-and-insert with WRITE lock
    let mut sessions = self.sessions.write().await;

    if !sessions.contains_key(&id) {
        // ID is unique - insert atomically
        sessions.insert(id.clone(), session);
        drop(sessions);  // Release lock immediately after insert
        break id;
    }

    // Collision detected (should never happen)
    drop(sessions);  // Release lock before logging
    collision_count += 1;
    log::error!(
        "UUID v4 collision #{collision_count} detected: {id}. \
         This indicates a serious problem with the RNG!"
    );

    if collision_count >= 10 {
        return Err(McpError::Other(anyhow::anyhow!(
            "Unable to generate unique session ID after 10 attempts. \
             System RNG may be compromised."
        )));
    }

    // Loop continues - will regenerate session with new ID
};

// session_id is now guaranteed unique and inserted
// Continue with rest of start_search logic...
```

### Key Implementation Details

1. **Move `validate_path()` before the loop** (line 100 → before line 76)
   - Path validation can fail, no point generating IDs if path is invalid
   - Only needs to run once, not in loop

2. **Create channels before the loop** (lines 103-106 → before line 76)
   - Channels need to be the same across collision retries
   - Clone them when building session

3. **Build session inside the loop**
   - Each collision retry needs fresh session with new ID
   - Session construction is cheap (~10 microseconds)

4. **Use write lock for check-and-insert atomically**
   - No read lock needed - write lock allows reading too
   - Hold write lock only during check and insert
   - Drop immediately after insert

5. **Clone channels in session construction**
   - `cancellation_tx.clone()` is cheap (just `Arc` increment)
   - `first_result_tx.clone()` is cheap (just `Arc` increment)

### Performance Impact

**Concern:** "Won't holding write lock for longer hurt performance?"

**Analysis:**
- **Before:** Read lock ~10ns, write lock ~10ns = 20ns total
- **After:** Write lock ~10ns (held during fast check + insert)
- **Session construction:** ~10 microseconds (10,000ns)

**BUT:** Session construction now happens **outside the lock hold time** in terms of other threads waiting. We build the session **before** acquiring the lock.

**Lock hold time comparison:**
- **Before:** Read lock held during `contains_key()` only (~50ns)
- **After:** Write lock held during `contains_key()` + `insert()` (~100ns)

**Net difference:** ~50ns per `start_search()` call - completely negligible.

**Contention:** Write lock means only one thread can check IDs at a time, but:
- ID generation + check takes ~100ns total
- Search setup (path validation, etc.) takes milliseconds
- Contention on this lock is not the bottleneck

---

## Definition of Done

The fix is complete when:

1. ✅ **Code changed:** Lines 76-137 in `src/search/manager/core.rs` follow the pattern above
2. ✅ **Session construction moved:** Before lock acquisition, inside the retry loop
3. ✅ **Atomic check-and-insert:** Both happen under the same write lock with no gap
4. ✅ **Path validation moved:** Happens before the ID generation loop
5. ✅ **Channels created once:** Before the loop, cloned in session construction
6. ✅ **Lock released promptly:** Immediately after insert, before logging
7. ✅ **Compilation succeeds:** No syntax errors, all types match
8. ✅ **Behavior unchanged:** Start search functionality works identically to before

### Verification Checklist

- [ ] Read the current code at lines 76-137
- [ ] Move path validation (line 100) to before line 76
- [ ] Move channel creation (lines 103-106) to before line 76
- [ ] Move session construction (lines 109-131) into the loop, before lock
- [ ] Change `read()` to `write()` in the lock acquisition
- [ ] Verify `contains_key()` and `insert()` happen under same lock hold
- [ ] Ensure lock is dropped immediately after insert
- [ ] Test that searches still work correctly

---

## Why Not Use `entry()` API?

**Alternative considered:**
```rust
use std::collections::hash_map::Entry;

let mut sessions = self.sessions.write().await;
match sessions.entry(id) {
    Entry::Vacant(e) => {
        e.insert(session);
        break id;
    }
    Entry::Occupied(_) => {
        // collision - retry
    }
}
```

**Why we don't use this:**
- HashMap `entry()` API consumes the key on `Occupied` case
- Would need to clone the ID to use it after checking
- The simple `contains_key() + insert()` pattern is clearer
- Both are equally correct and performant

**Our approach is simpler and more readable.**

---

## Related Files

- [`src/search/manager/core.rs`](../src/search/manager/core.rs) - Main file to modify
- [`src/search/types.rs`](../src/search/types.rs) - SearchSession struct definition
- [Research notes](../tmp/research.txt) - Analysis documentation

---

## References

**Rust Concurrency Patterns:**
- Check-then-act requires atomicity: single lock hold for both operations
- Read locks don't prevent writes - write lock needed for atomic check-and-insert
- `RwLock::write()` allows reading too, so no need for read lock first

**Performance Considerations:**
- Lock hold time: ~100ns (negligible compared to ~10ms total search setup)
- Session construction: ~10μs (all Arc clones, no I/O)
- UUID generation: ~1μs (crypto-quality random)

**Probability vs. Correctness:**
Even though UUID collisions are impossibly rare, correct concurrent code requires proper atomicity regardless of probability.
