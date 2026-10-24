# Runestone User Manual (Rune Web UI)

## Overview

**Runestone** is a local, read-only web interface for visually browsing Rune repository history. Served by the `rune-web` crate and embedded directly into the `rune-cli` binary, it provides a visual way to explore commit timelines, file trees at any point in history, syntax-highlighted code, and unified diffs.

Runestone operates entirely **offline** and requires zero external network access (no calls to GCS, remote servers, or external CDNs). All data is read directly from your local `.rune/objects` store.

---

## Key Features

- **Commit Timeline:** Chronological commit history for local and tracking branches, displaying authors, dates, messages, and commit hashes.
- **Interactive Tree Browser:** Click any commit or directory to browse repository hierarchies as they existed at that snapshot.
- **Syntax-Highlighted File Viewer:** View file contents directly in your browser with automatic syntax coloring for code and notices for binary files.
- **Unified Diff Viewer:** Inspect file changes, added lines (`+`), and removed lines (`-`) between a commit and its parent.
- **Branch Overview:** View all local and remote-tracking branch pointers.
- **Localhost Security Sandboxing:** Binds exclusively to loopback (`127.0.0.1`), enforces strict Host and Origin validation against DNS rebinding, and rejects traversal or path escaping attempts.

> [!NOTE]
> Runestone is strictly **read-only**. You cannot commit, merge, push, or mutate your repository through the web interface. All repository mutations must be executed through `rune-cli`.

---

## Launching the Web Interface

### Basic Command

Navigate to any directory within your initialized Rune repository and run:

```bash
# Start server on default port 8080
rune-cli web

# Start server on a custom port
rune-cli web --port 3000
```

Console output will display:
```
Runestone starting on http://127.0.0.1:8080
```

Open `http://127.0.0.1:8080` in your web browser.

---

## Route Overview

Runestone provides the following HTTP `GET` endpoints:

| Route | Description |
|---|---|
| `GET /` | Redirects to the current branch commit timeline or `/commits`. |
| `GET /commits` | Main commit history timeline. |
| `GET /tree` | Working tree root directory view at the current `HEAD`. |
| `GET /commit/:hash/tree` | Root directory tree for a specific commit hash. |
| `GET /commit/:hash/tree/*path` | Subdirectory tree browsing at a given commit path. |
| `GET /commit/:hash/blob/*path` | File content viewer with syntax highlighting. |
| `GET /commit/:hash/diff` | Unified diff viewer comparing commit changes against its parent. |
| `GET /refs` | Overview of all branch references and HEAD status. |

---

## Security Model & Protections

Runestone incorporates multiple layers of security to protect developers:

1. **Loopback Only:** Binds strictly to `127.0.0.1`. It does not listen on `0.0.0.0`, preventing access from other machines on your local network.
2. **Anti-DNS Rebinding Middleware:** Incoming HTTP requests pass through `middleware::validate_host`. The `Host` header must resolve to `localhost` or `127.0.0.1`. Requests with external hostnames or malicious cross-site `Origin` headers are rejected with HTTP 403 Forbidden.
3. **Strict Path Sandboxing:** Route parameters for trees and blobs cannot contain `..`, absolute path prefixes (`/`, `\`), or URL-encoded traversal payloads.
4. **Decompression Bomb Protection:** Binary and text object readers limit decompression sizes, preventing memory exhaustion when inspecting adversarial commits.

---

## Tips & Workflow

> [!TIP]
> **Live Local Inspection:** Because Runestone reads directly from `.rune/objects` on demand, you can leave `rune-cli web` running in a terminal tab while you continue coding, staging, and committing in another tab. Refresh your browser page to instantly see your newest commits and diffs!

---

## Related Documentation

- [Rune CLI User Manual](rune_cli_manual.md)
- [ADR-004: Offline Read-Only Web UI Architecture](decisions/ADR-004-offline-read-only-web-ui-runestone.md)
- [Runestone Technical Requirements (TRD)](Runestone_TRD.md)
- [Runestone Product Requirements (PRD)](Runestone_PRD.md)
