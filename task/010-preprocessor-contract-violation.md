# Preprocessor Contract Violation Creates Runtime Error for Programming Bug

## Location
`src/search/rg/search.rs:266-274`

## Severity
Medium (Programming error becomes runtime error)

## Issue Description
The `search_preprocessor` function uses `ok_or_else()` to handle a `None` case that should be a compile-time error:

```rust
/// Search the given file path by first asking the preprocessor for the
/// data to search instead of opening the path directly.
///
/// PRECONDITION: This function must only be called when preprocessor is Some,
/// as guaranteed by `should_preprocess()` check.
fn search_preprocessor(&mut self, path: &Path) -> io::Result<()> {
    use std::{fs::File, process::Stdio};

    // should_preprocess() ensures preprocessor is Some before calling this
    // If it's None, this indicates a programming error in the contract
    let bin = self.config.preprocessor.as_ref()
        .ok_or_else(|| io::Error::other(  // Line 271-273
            "BUG: search_preprocessor called with None preprocessor - should_preprocess() contract violated"
        ))?;
    // ...
}
```

**The Problem:**
1. **Documented precondition:** "must only be called when preprocessor is Some"
2. **Runtime check:** `ok_or_else()` creates I/O error at runtime
3. **Contract violation:** If `None`, it's a programming bug, not I/O error
4. **Wrong error type:** Returns `io::Result` for logic error

## Real-World Impact

### Scenario: Bug in Call Site
If someone modifies the code incorrectly:

```rust
// Buggy code
fn search(&mut self, haystack: &Haystack) -> io::Result<()> {
    let path = haystack.path();

    // WRONG: Forgot to check should_preprocess()
    self.search_preprocessor(path)?;  // Contract violation!
}
```

**What happens now:**
1. Returns `io::Error` with "BUG: ..." message
2. Error bubbles up through I/O error handling
3. User sees cryptic error in search results
4. Developer doesn't get immediate feedback during testing

**What should happen:**
1. Debug assertion fails immediately during development
2. Release build uses `unreachable_unchecked` for zero overhead
3. Clear distinction between programmer errors and I/O errors

### Scenario: Refactoring Goes Wrong
```rust
// Before refactoring:
if self.should_preprocess(path) {
    self.search_preprocessor(path)?;
}

// After refactoring (BUGGY):
match self.config.preprocessor {
    Some(_) => self.search_preprocessor(path)?,  // Looks safe...
    None => self.search_path(path)?,
}

// But config was just mutated to None by another thread!
// Runtime error instead of catching bug early
```

## Root Cause
Mixing error handling strategies:
- **Documented contract:** Precondition (caller must ensure)
- **Defensive programming:** Runtime check (assuming contract might break)
- **Wrong error type:** Logic error as I/O error

This confuses:
1. When caller should check
2. When function should check
3. What kind of error it is

## Recommended Fix: Debug Assertions

```rust
fn search_preprocessor(&mut self, path: &Path) -> io::Result<()> {
    use std::{fs::File, process::Stdio};

    // In debug builds: catch contract violations immediately
    debug_assert!(
        self.config.preprocessor.is_some(),
        "BUG: search_preprocessor called with None preprocessor. \
         This is a programming error - should_preprocess() should \
         guarantee preprocessor is Some before calling this function."
    );

    // In release builds: use unreachable_unchecked for zero overhead
    // SAFETY: Precondition documented in contract - caller MUST call
    // should_preprocess() first. If this is violated, it's undefined
    // behavior (but only reachable through programmer error).
    let bin = unsafe {
        self.config.preprocessor.as_ref().unwrap_unchecked()
    };

    let mut cmd = std::process::Command::new(bin);
    // ...rest of function...
}
```

**Benefits:**
1. **Development:** Catches bugs immediately with clear panic
2. **Release:** Zero overhead (compiled away)
3. **Clear semantics:** This is a precondition, not an error case
4. **Correct error type:** Panics (logic error) vs I/O errors (runtime error)

## Alternative: Type System Enforcement

Make it impossible to call incorrectly using types:

```rust
// Newtype to guarantee preprocessor exists
struct PreprocessorCommand {
    bin: PathBuf,
}

impl SearchWorker<W> {
    // Returns Some only if preprocessor is configured
    fn preprocessor_command(&self, path: &Path) -> Option<PreprocessorCommand> {
        if !self.should_preprocess(path) {
            return None;
        }

        self.config.preprocessor.as_ref().map(|bin| PreprocessorCommand {
            bin: bin.clone(),
        })
    }

    // Only accepts PreprocessorCommand - can't call without preprocessor
    fn search_with_preprocessor(
        &mut self,
        path: &Path,
        cmd: PreprocessorCommand,
    ) -> io::Result<()> {
        // No need to check - type guarantees it exists
        let mut process = std::process::Command::new(&cmd.bin);
        // ...
    }

    fn search(&mut self, haystack: &Haystack) -> io::Result<()> {
        let path = haystack.path();

        // Type system enforces the check
        if let Some(preprocessor) = self.preprocessor_command(path) {
            self.search_with_preprocessor(path, preprocessor)?;
        } else {
            self.search_path(path)?;
        }

        Ok(())
    }
}
```

**Benefits:**
1. **Compile-time safety:** Can't call wrong function
2. **Zero runtime overhead:** Type erased in release
3. **Self-documenting:** Function signature shows requirement
4. **Refactoring-safe:** Type checker catches mistakes

**Drawbacks:**
- More code
- Slightly more complex API

## Error Handling Philosophy

### When to use each approach:

**1. Debug Assertions (Preconditions)**
```rust
debug_assert!(index < vec.len(), "Index out of bounds");
```
- Use for: Preconditions that caller must guarantee
- Examples: Array bounds, non-null pointers, state invariants

**2. Type System (Compile-Time Guarantees)**
```rust
fn process_nonempty(items: NonEmpty<Vec<T>>) { }
```
- Use for: Invariants that can be encoded in types
- Examples: Non-empty collections, valid states

**3. Runtime Errors (External Failures)**
```rust
File::open(path).map_err(|e| format!("Cannot open {}: {}", path, e))?
```
- Use for: I/O errors, network failures, invalid user input
- Examples: File not found, network timeout, parse errors

**4. Panics (Unrecoverable Bugs)**
```rust
unreachable!("Bug: all cases should be handled")
```
- Use for: Programmer errors that should never happen
- Examples: Unreachable code, violated invariants

## Current Code Violates Separation

```rust
// Mixes categories:
self.config.preprocessor.as_ref()
    .ok_or_else(|| io::Error::other("BUG: ..."))  // Programming error as I/O error
```

Should be either:
- Debug assertion (recommended for performance)
- Type system enforcement (recommended for safety)
- Never runtime I/O error

## Impact Assessment

| Aspect | Current | With Debug Assert | With Types |
|--------|---------|------------------|------------|
| Bug detection | Late (runtime) | Early (dev time) | Earliest (compile time) |
| Error clarity | Confusing | Clear panic | Won't compile |
| Runtime overhead | Small | Zero | Zero |
| API complexity | Simple | Simple | Moderate |

## Testing Recommendation

```rust
#[test]
#[should_panic(expected = "BUG: search_preprocessor called with None")]
fn test_preprocessor_contract_violation() {
    let mut worker = SearchWorker::new();

    // Deliberately violate contract
    worker.config.preprocessor = None;

    // Should panic in debug builds
    worker.search_preprocessor(Path::new("test.txt")).unwrap();
}

// For type-based approach:
#[test]
fn test_preprocessor_type_safety() {
    let worker = SearchWorker::new();

    // Won't compile without preprocessor command
    // worker.search_with_preprocessor(path, ???);

    // Must go through proper API
    if let Some(cmd) = worker.preprocessor_command(path) {
        worker.search_with_preprocessor(path, cmd).unwrap();
    }
}
```

## Recommendation
Use **debug assertion approach**:
- Minimal code change
- Clear semantics (precondition violation)
- Zero overhead in release
- Catches bugs early in development

Consider **type-based approach** for major refactoring or if this area sees frequent bugs.
