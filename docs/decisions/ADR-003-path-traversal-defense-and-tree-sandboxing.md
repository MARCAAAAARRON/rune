# ADR-003: Path Traversal Defense and Strict Tree Sandboxing

## Status
Accepted

## Date
2026-10-18

## Context
Version control systems accept untrusted data when developers clone or fetch repositories from external contributors or public remotes. Historical CVEs in Git (e.g., CVE-2014-9390, CVE-2018-11235, CVE-2024-32002) have shown that maliciously crafted tree entries can escape the working tree directory, overwrite arbitrary system files, or execute arbitrary code (e.g., via `.git/hooks` traversal or symlink indirection).

Rune must provide built-in defense against path traversal, symlink escapes, and null-byte injection during checkout and file operations.

## Decision
We enforce multi-layered path validation and sandboxing within `rune-core`:
1. **Tree Entry Component Validation (`validate_path_component`):**
   - Individual tree entry names must strictly not contain path separators (`/`, `\`). Directories are represented strictly as nested `Tree` objects.
   - Entry names must strictly not equal relative navigation names (`.` or `..`).
   - Entry names must strictly not contain null bytes (`\0`).
2. **Symlink Target Inspection:**
   - Symlink blobs are validated upon checkout: targets cannot start with a slash (`/`, `\`) or contain relative traversal (`..`). Symlinks that attempt to escape outside the repository working tree are rejected with a `SecurityViolation` error.
3. **Canonical Path Sandboxing:**
   - When staging (`rune-cli add`) or extracting (`rune-cli checkout`), file paths are canonicalized and verified against the canonical repository root path.
4. **Decompression Bomb Protection:**
   - Zstandard decompression limits memory consumption to prevent unbounded allocation attacks from adversarial compressed blobs.

## Alternatives Considered

### Permissive Path Normalization (Sanitizing after parsing)
- **Pros:** Forgiving of malformed third-party trees; automatically strips leading slashes or `..`.
- **Cons:** Normalization routines frequently have edge cases across different operating systems (Windows path quirks like `CON`, `NUL`, NTFS alternate data streams, or trailing dots/spaces).
- **Rejected because:** Strict rejection (failing fast with `SecurityViolation`) prevents zero-day normalization bypasses.

### Relying on OS File Permissions
- **Pros:** No application-layer path checking required.
- **Cons:** If an attacker writes to `~/.bashrc` or `C:\Users\...`, the OS will permit it if Rune is running under that user's privileges.
- **Rejected because:** Application-level sandboxing is essential to guarantee user safety when inspecting untrusted repositories.

## Consequences
- **Security Assurance:** Checkout of untrusted repositories cannot overwrite files outside the working tree or overwrite `.rune` internals.
- **Strict Format:** Any commit containing malicious tree entries is immediately rejected during checkout or tree walking.
- **Cross-Platform Consistency:** Both Unix-style and Windows-style path separators are rejected uniformly across all host operating systems.
