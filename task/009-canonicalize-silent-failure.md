# Silent Canonicalize Failure Masks Path Resolution Issues

## Location
`src/validation.rs:189-206`

## Severity
Medium (Could hide filesystem issues)

## Issue Description
When `canonicalize()` fails, the code silently falls back to the absolute path without logging or informing the user:

```rust
// Check if path exists
match fs::metadata(&absolute).await {
    Ok(_) => {
        // If path exists, resolve any symlinks
        match fs::canonicalize(&absolute).await {
            Ok(canonical) => Ok(canonical),
            Err(_) => Ok(absolute),  // Line 194: SILENT FALLBACK
        }
    }
    Err(_) => {
        // Path doesn't exist - validate parent directories
        if validate_parent_directories(&absolute).await {
            Ok(absolute)
        } else {
            // Return absolute anyway for operations that create paths
            Ok(absolute)  // Line 203: Another silent fallback
        }
    }
}
```

**Problems:**
1. **Symlink loops:** If `canonicalize()` fails due to circular symlinks, we proceed with the unresolved path
2. **Permission issues:** Can't read symlink target, but proceed anyway
3. **Broken symlinks:** Return path to broken symlink instead of failing
4. **No user feedback:** User doesn't know path resolution failed

## Real-World Impact

### Scenario 1: Circular Symlink Loop
```bash
$ ln -s /a /b
$ ln -s /b /a
```

User tries to read `/a/file.txt`:
1. `metadata(/a)` succeeds (symlink exists)
2. `canonicalize(/a)` fails (circular symlink)
3. Returns `/a` as-is
4. Later file operation fails with cryptic error

**Better behavior:**
- Detect circular symlink
- Return error: "Path contains circular symlink: /a -> /b -> /a"
- User understands the problem

### Scenario 2: Permission Denied on Symlink
```bash
$ ln -s /root/secret /public/link
$ chmod 000 /root/secret
```

User tries to read `/public/link`:
1. `metadata(/public/link)` succeeds (can stat the symlink itself)
2. `canonicalize(/public/link)` fails (permission denied)
3. Returns `/public/link` as-is
4. Later read fails with "permission denied"

**Better behavior:**
- Log warning: "Cannot resolve symlink /public/link: permission denied"
- User knows to check permissions on symlink target

### Scenario 3: Broken Symlink
```bash
$ ln -s /nonexistent /public/broken
```

User tries to read `/public/broken`:
1. `metadata(/public/broken)` succeeds (broken symlink still has metadata)
2. `canonicalize(/public/broken)` fails (target doesn't exist)
3. Returns `/public/broken` as-is
4. Later operations confusing

**What happens:** File operations fail on `/public/broken` with "not found"
**User sees:** "File `/public/broken` not found" (but file exists as symlink!)
**Should see:** "Symlink `/public/broken` is broken (target `/nonexistent` not found)"

## Root Cause
Using catch-all `Err(_)` pattern that discards error information. All failure modes are treated identically.

## Recommended Fix: Log Warnings and Preserve Error Info

```rust
match fs::metadata(&absolute).await {
    Ok(metadata) => {
        // If path exists, try to resolve symlinks
        match fs::canonicalize(&absolute).await {
            Ok(canonical) => Ok(canonical),
            Err(e) => {
                // Log warning with specific error
                use std::io::ErrorKind;
                match e.kind() {
                    ErrorKind::NotFound => {
                        // Broken symlink
                        warn!(
                            "Broken symlink detected: {} (target not found)",
                            absolute.display()
                        );
                    }
                    ErrorKind::PermissionDenied => {
                        // Can't access symlink target
                        warn!(
                            "Cannot resolve symlink {} due to permissions",
                            absolute.display()
                        );
                    }
                    ErrorKind::FilesystemLoop => {
                        // Circular symlink
                        warn!(
                            "Circular symlink detected: {}",
                            absolute.display()
                        );
                    }
                    _ => {
                        // Other error
                        warn!(
                            "Cannot canonicalize path {}: {}",
                            absolute.display(),
                            e
                        );
                    }
                }

                // For most operations, falling back to absolute is OK
                // But log it so issues are visible
                Ok(absolute)
            }
        }
    }
    Err(e) => {
        // Path doesn't exist
        if validate_parent_directories(&absolute).await {
            Ok(absolute)
        } else {
            // Even this fallback should be logged
            debug!(
                "Path {} and ancestors don't exist, but allowing for create operations",
                absolute.display()
            );
            Ok(absolute)
        }
    }
}
```

## Alternative: Fail on Broken Symlinks

For security-sensitive operations, consider failing instead of falling back:

```rust
match fs::canonicalize(&absolute).await {
    Ok(canonical) => Ok(canonical),
    Err(e) => {
        // Check if it's a symlink that failed to resolve
        if metadata.is_symlink() {
            // This is a broken/unresolvable symlink - fail explicitly
            return Err(McpError::InvalidPath(format!(
                "Cannot resolve symlink {}: {}",
                absolute.display(),
                e
            )));
        }
        // Not a symlink, fall back to absolute is OK
        Ok(absolute)
    }
}
```

**When to use:**
- Security-sensitive operations (delete, move)
- When symlink target matters
- When user should fix broken symlinks

**When not to use:**
- File creation (target doesn't exist yet)
- When absolute path is sufficient
- Backward compatibility concerns

## Impact Assessment

| Issue | Current | With Logging | With Failure |
|-------|---------|--------------|--------------|
| Circular symlinks | Silent, later error | Logged, expected | Immediate error |
| Broken symlinks | Confusing errors | Clear warning | Clear error |
| Permission denied | Cryptic failure | Logged reason | Clear error |
| Debugging | Hard | Easy | Easy |
| User experience | Poor | Good | Best (but stricter) |

## Security Consideration

Broken symlinks can be a security issue:
1. Attacker creates symlink to sensitive file
2. Sensitive file is deleted
3. Symlink now broken but still points to location
4. If file is recreated, attacker gains access

**Mitigation:** Fail on broken symlinks for write operations.

## Testing Recommendation

```rust
#[tokio::test]
async fn test_circular_symlink_handling() {
    let temp = tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");

    // Create circular symlinks
    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();

    // Should either fail or log warning
    let result = validate_path(&a.to_string_lossy(), &config).await;

    // With logging fix: should succeed with warning logged
    assert!(result.is_ok());
    // Verify warning was logged (requires log capture)

    // With failure fix: should return error
    // assert!(result.is_err());
    // assert!(matches!(result.unwrap_err(), McpError::InvalidPath(_)));
}

#[tokio::test]
async fn test_broken_symlink_handling() {
    let temp = tempdir().unwrap();
    let link = temp.path().join("broken_link");
    let target = temp.path().join("nonexistent");

    // Create broken symlink
    std::os::unix::fs::symlink(&target, &link).unwrap();

    // Should handle broken symlink gracefully
    let result = validate_path(&link.to_string_lossy(), &config).await;

    // Test behavior based on chosen fix
}
```

## Recommendation
**Implement logging version first:**
- Backward compatible
- Improves debugging
- Users can see warnings
- Can upgrade to failure version later if needed

**Consider failure version for:**
- Security-critical operations
- Major version bump
- When breaking changes are acceptable
