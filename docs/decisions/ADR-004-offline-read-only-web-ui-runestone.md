# ADR-004: Offline Read-Only Web Interface (Runestone) with Loopback Binding

## Status
Accepted

## Date
2026-10-20

## Context
Developers frequently need visual exploration of repository commit timelines, tree hierarchies, syntax-highlighted files, and branch diffs without leaving their local environment. While external SaaS services (like GitHub or GitLab) provide rich web interfaces, they require network connectivity and pushing private code to external clouds.

Rune requires a lightweight, integrated web interface that can run locally, operate completely offline, and maintain strict security boundaries.

## Decision
We designed and implemented **Runestone** (`rune-web`), an embedded Axum-based web server:
1. **Strictly Read-Only:** Runestone exposes only HTTP `GET` endpoints (`/`, `/commits`, `/commit/:hash/tree`, `/commit/:hash/blob/*path`, `/commit/:hash/diff`, `/refs`). No mutating routes exist, eliminating CSRF vulnerabilities.
2. **Loopback Binding (`127.0.0.1`):** The server binds exclusively to `127.0.0.1` by default and never listens on `0.0.0.0` or public network interfaces.
3. **Host and Origin Validation Middleware (`middleware::validate_host`):**
   - Validates incoming `Host` headers to match `localhost` or `127.0.0.1` (with optional port).
   - Validates `Origin` headers (if present) to prevent DNS rebinding attacks executed by malicious websites in the developer's web browser.
4. **Embedded Static Assets:** Uses `rust-embed` to compile CSS, templates, and static assets directly into the binary, ensuring zero external asset CDN dependencies and offline functionality.
5. **Direct Integration with `rune-cli`:** Exposed as `rune-cli web [--port <port>]`, removing the need for a separate binary installation.

## Alternatives Considered

### Dedicated Desktop GUI (e.g., Tauri, Electron, or GTK)
- **Pros:** Native desktop window framing and OS system tray integration.
- **Cons:** Significantly increases binary size, build times, platform dependencies (C/C++ runtime headers, WebView libraries), and cross-compilation complexity.
- **Rejected because:** An embedded local web server served to the user's existing default web browser provides instant zero-dependency cross-platform visualization.

### Dynamic Client-Side Single Page App (SPA with REST API)
- **Pros:** Rich client-side transitions.
- **Cons:** Requires Node.js build steps, heavy JavaScript dependencies, and larger asset bundles.
- **Rejected because:** Server-side rendered templates via Axum and Askama compile into native Rust bytecode with zero runtime JS runtime dependencies and instantaneous page loads.

## Consequences
- **Zero Configuration:** Developers can launch `rune-cli web` and view full commit graphs, diffs, and trees in seconds.
- **Security Boundaries:** Protection against DNS rebinding and cross-origin exploitation is built directly into middleware.
- **Low Footprint:** Negligible idle CPU and memory consumption.
