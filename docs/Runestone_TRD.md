# Technical Requirements Document (TRD)
## Runestone — A Local Web UI for Rune

**Version:** 1.0
**Status:** Draft
**Owner:** Cliny
**Related documents:** Runestone_PRD.md, Runestone_Implementation_Plan.md
**Parent project TRD:** Rune_TRD.md (object model, diff engine, and commit graph traversal are reused, not reimplemented)

---

## 1. Architecture Overview

Runestone is a thin, read-only presentation layer on top of Rune's existing core engine (`rune-core`, per the Rune TRD's crate split). It adds no new persistence, no new object formats, and no network client of any kind — it is purely: **HTTP request → read local `.rune/objects` via `rune-core` → render HTML → HTTP response.**

```
┌─────────────────────────────────────────────┐
│  Browser (localhost only)                    │
└───────────────────┬───────────────────────────┘
                    │ HTTP (loopback)
┌───────────────────▼───────────────────────────┐
│  rune-web (Axum server)                       │
│   - Route handlers (commits, tree, diff, refs)│
│   - Templates (server-rendered HTML)          │
│   - Output sanitization layer                 │
├─────────────────────────────────────────────┤
│  rune-core (shared with rune-cli, read paths) │
│   - Object store reader                       │
│   - Commit graph traversal                    │
│   - Diff engine                               │
├─────────────────────────────────────────────┤
│  Local filesystem: .rune/objects, refs/       │
└─────────────────────────────────────────────┘
```

Crucially: **rune-web never writes**. Every code path it touches in `rune-core` is a read path already exercised (and hardened, per Rune TRD §9) by the CLI's own `log`/`diff`/`checkout`-adjacent logic.

## 2. Workspace Structure

Added to the existing Cargo workspace (alongside `rune-core`, `rune-cli`):

```
rune-web/
  src/
    main.rs            # or invoked as `rune web` from rune-cli
    routes/
      commits.rs
      tree.rs
      diff.rs
      refs.rs
    templates/          # Askama templates (compiled, type-checked at build time)
    static/              # CSS, minimal JS — embedded via rust-embed
```

**Subcommand vs. separate binary:** implement as `rune web`, a subcommand of the existing `rune-cli` binary, rather than a second binary. This preserves the "one binary, no separate install" property from the Rune PRD's NFRs, and avoids duplicating argument parsing / config loading.

## 3. Routes (v1 scope)

| Route | Purpose |
|---|---|
| `GET /` | Redirect to current branch's commit timeline |
| `GET /branch/:name/commits` | Paginated commit timeline for a branch |
| `GET /commit/:hash` | Commit detail (message, author, parent links, changed-files summary) |
| `GET /commit/:hash/tree` | File tree at this commit |
| `GET /commit/:hash/tree/*path` | Subdirectory listing |
| `GET /commit/:hash/blob/*path` | File content view (syntax-highlighted or binary notice) |
| `GET /commit/:hash/diff` | Diff vs. parent (or vs. a second `?against=` param for arbitrary comparison) |
| `GET /refs` | Branch/ref overview |

All routes are **GET-only** in v1 — there is no mutating endpoint anywhere in Runestone, which meaningfully simplifies the security model (no CSRF concern in the traditional sense, since there's nothing to forge a state-changing request *to*).

## 4. Rendering Approach

- **Server-rendered HTML via Askama** (compile-time-checked templates, no runtime template-injection risk the way a string-interpolated templating approach could have) rather than a client-side JS framework. Consistent with the "single binary, minimal dependencies" philosophy carried over from Rune itself.
- **Progressive enhancement with a small amount of vanilla JS or htmx** for things like expand/collapse tree nodes or paginated commit loading — avoids a full SPA build toolchain (no Node/npm/webpack requirement to *build* the project, only `cargo build`).
- **Syntax highlighting**: `syntect` crate (used by tools like `bat`), run server-side at render time — output is escaped HTML, not executed script, which matters for the XSS discussion in §7.1.
- **Diff rendering**: reuses Rune's existing diff engine (`similar` crate output) and renders it as a side-by-side or unified HTML view — no new diff algorithm needed.

## 5. Data Flow Example — Viewing a File

1. Browser requests `GET /commit/<hash>/blob/src/main.rs`.
2. Handler resolves `<hash>` to a commit object (reusing `rune-core`'s object reader — this is the same trusted, hash-verifying read path described in Rune TRD §9.2).
3. Handler walks the commit's tree to resolve `src/main.rs` to a blob hash, validating each path segment against the same traversal rules as Rune's checkout logic (Rune TRD §9.1) — **path safety matters here too**, even though nothing is being written to disk (see §7.4).
4. Blob content is decompressed, checked against a size ceiling (§7.3) before full buffering.
5. Content is either passed to `syntect` for highlighting (text) or replaced with a "binary file, N bytes" notice (§7.5) — the decision is based on content sniffing, not file extension alone.
6. Highlighted/escaped output is rendered into the Askama template and returned.

## 6. Technology Stack

| Concern | Choice | Rationale |
|---|---|---|
| Web framework | `axum` | Built on `tokio`, which Rune's remote/GCS work already depends on (Rune TRD §5) — shares the async runtime instead of introducing a second one (e.g. Actix's own runtime model) |
| Templating | `askama` | Compile-time-checked templates; syntax-error-class bugs caught at `cargo build`, not at request time; avoids runtime template-injection risk inherent to string-based templating |
| Syntax highlighting | `syntect` | Mature, widely used (powers `bat`), reasonable performance for on-demand server-side highlighting |
| Static assets | `rust-embed` | Bundles CSS/JS into the compiled binary — preserves the single-binary distribution property |
| Progressive interactivity | `htmx` (optional, minimal) or vanilla JS | Avoids a full SPA framework and its build toolchain, in line with keeping this a "Rust project," not a "Rust + JS project" |
| Diff rendering | Reuses `similar` output from `rune-core` | No new diff logic — just an HTML view over data Rune's CLI already computes |

## 7. Security & Vulnerability Analysis

Being "local-only" reduces the attack surface but does **not** eliminate it — this section treats Runestone with the same rigor as Rune's own TRD §9, since a browser rendering untrusted repository content is a real client, not a trusted internal tool.

### 7.1 Stored XSS via repository content
**Threat:** Commit messages, author names, file contents, and file/directory names are all attacker-controlled if the repository being browsed was cloned/pulled from an untrusted source. If any of these are rendered into HTML without escaping, a crafted commit message like `<script>...</script>` executes in the viewer's browser.
**Mitigation:** Askama auto-escapes template variables by default — this must never be bypassed (no `|safe` filters on repository-derived content, ever). Syntax-highlighted code output from `syntect` must itself be escaped correctly (verify the crate's HTML output escapes embedded `<`/`>`/`&` — do not assume). Treat every field sourced from the object store as untrusted input at render time, full stop.

### 7.2 DNS rebinding against the local server
**Threat:** This is the security issue specific to "local web server + browser" that a pure CLI tool never faces. A malicious website the user visits in the *same browser* can use DNS rebinding (a domain that initially resolves to an external IP, then rebinds to `127.0.0.1`) to make requests against Runestone's local server from JavaScript running on that malicious page — potentially reading repository contents out through the browser.
**Mitigation:** Validate the `Host` header on every request and reject anything that isn't `localhost`/`127.0.0.1` with the expected port; additionally validate `Origin`/`Referer` headers reject cross-origin requests. Binding to loopback alone (`127.0.0.1`, not `0.0.0.0`) is necessary but **not sufficient** — DNS rebinding specifically defeats "it's only on localhost" as a complete defense, which is why header validation is called out as its own requirement here rather than assumed covered by the bind address.

### 7.3 Resource exhaustion via large files/diffs
**Threat:** A very large file or a diff between two very different large files could cause slow rendering, excessive memory use, or a hung request — same underlying class of issue as Rune's decompression-bomb concern (Rune TRD §9.4), now surfaced through an HTTP request instead of a CLI command.
**Mitigation:** Enforce a maximum file size before attempting syntax highlighting or diff rendering (show a "file too large to display, N bytes" notice past the threshold); paginate commit lists and large diffs rather than rendering everything in one response.

### 7.4 Path traversal via route parameters
**Threat:** The `*path` wildcard segments in tree/blob routes are user-supplied (via the URL) even though they're ultimately used to navigate an in-memory tree object, not the filesystem directly — but if any code path ever uses these to touch the real filesystem (e.g. caching rendered output to disk, or resolving static assets), the same `..`-traversal risk from Rune TRD §9.1 applies.
**Mitigation:** Treat route-supplied paths as tree-object lookup keys only, never as filesystem paths; apply the same path-segment validation Rune's checkout logic already uses (Rune TRD §9.1) if that logic is ever reused or shared for consistency; if any disk caching is added later, canonicalize and validate before every write.

### 7.5 Binary file mishandling
**Threat:** Attempting to render an arbitrary binary file as UTF-8 text can produce garbled output, and in pathological cases, extremely large "text" (e.g. a binary file that happens to decode as an enormous string of certain byte patterns) feeding into the highlighter.
**Mitigation:** Content-sniff before rendering (check for null bytes / invalid UTF-8 in a leading chunk, not just trust a file extension); render a clear "binary file" notice with size and a download-not-render option, rather than attempting highlighting.

### 7.6 Information exposure beyond the intended repository
**Threat:** If any handler logic is ever misconfigured to resolve paths outside `.rune/` (e.g. a bug in blob-serving logic), the local server could inadvertently expose arbitrary files on the user's machine to anything that can reach it (compounding §7.2 if header validation is also missing).
**Mitigation:** All file/object reads go through `rune-core`'s object store abstraction, never raw filesystem paths derived from user input — there should be no code path in `rune-web` that opens a file by a string built from request data.

### 7.7 Denial of service via repeated expensive requests
**Threat:** Since there's no auth and no rate limiting, a malicious local process (or a rebinding attack per §7.2 that gets past header checks) could hammer expensive routes (large diffs, deep tree walks) repeatedly.
**Mitigation:** This is a low-severity concern for a single-user local tool (impact is limited to your own machine's resources), but a simple per-IP or global request-rate cap is cheap insurance, especially combined with §7.2's defenses.

### 7.8 Dependency supply-chain risk
**Threat:** `axum`, `syntect`, `askama`, and their transitive dependencies are all attack surface — a compromised crate could affect a tool that reads potentially-untrusted repository content.
**Mitigation:** Same discipline as the parent Rune project: pin versions, review `cargo audit` output as part of CI, prefer well-maintained/widely-used crates (all four listed here qualify).

## 8. Testing Strategy

- **Unit tests**: route handlers tested against fixture repositories (reuse Rune's test-fixture pattern from Rune TRD §7).
- **Rendering tests**: snapshot-test rendered HTML for known inputs, including adversarial ones (a commit message containing `<script>`, a filename containing traversal sequences, a binary file, an empty repo).
- **Security-specific tests**: assert a request with a forged `Host`/`Origin` header is rejected (§7.2); assert oversized file/diff requests return the "too large" notice rather than hanging or OOMing (§7.3); assert repository content is never reflected unescaped in a response (§7.1) — this last one is worth an explicit automated check, not just manual review, since it's easy to regress silently.
- **Manual browser testing**: confirm no outbound network requests are ever made by the served pages (check browser devtools network tab against a repo with a maliciously-named "file" designed to look like an external resource reference).

## 9. Pros and Cons — Technical Trade-offs

### Pros
- Reuses Rune's already-hardened read paths (object decoding, hash verification, commit graph traversal) instead of reimplementing them — less new code, less new risk.
- Compile-time-checked templates (Askama) eliminate an entire class of template-injection bugs before they can ship.
- Single binary, single async runtime shared with the rest of Rune — no toolchain fragmentation.
- Read-only, no-auth, no-database design keeps the security model genuinely simpler than a typical web app, provided §7.1/§7.2 are taken seriously.

### Cons
- "Local-only" is a weaker security boundary than it sounds, specifically because of browser-based attacks like DNS rebinding (§7.2) — this needs explicit, tested defenses, not just an assumption that binding to loopback is enough.
- Server-side syntax highlighting (`syntect`) adds a non-trivial dependency and CPU cost per file view compared to, say, shipping highlighting to the client — acceptable trade-off for staying JS-toolchain-free, but worth naming as a cost.
- No caching layer in v1 means repeated views of the same large file/diff re-render from scratch every time — fine for typical repo sizes, a real limitation for very large histories.
- Being read-only is the right choice for v1 safety and scope, but it does mean Runestone can never fully replace even basic GitHub-UI workflows (e.g. no ability to stage/commit from the browser) — purely a viewer, by design.

## 10. Appendix — Shared vs. New Code

| Capability | Source |
|---|---|
| Object decoding, hash verification | Reused from `rune-core` (Rune TRD §2–3.1) |
| Commit graph traversal | Reused from `rune-core` (Rune TRD §3.4) |
| Diff computation | Reused from `rune-core` (Rune TRD §3.2) |
| Path-safety validation | Reused pattern from Rune TRD §9.1, applied to route params |
| HTTP routing, handlers | New (`rune-web`) |
| HTML templates, static assets | New (`rune-web`) |
| Syntax highlighting integration | New (`rune-web`) |
| Host/Origin validation middleware | New (`rune-web`) — no equivalent need existed in the CLI-only tool |
