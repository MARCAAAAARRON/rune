# Product Requirements Document (PRD)
## Runestone — A Local Web UI for Rune

**Version:** 1.0
**Status:** Draft
**Owner:** Cliny
**Related documents:** Runestone_TRD.md, Runestone_Implementation_Plan.md
**Parent project:** Rune (see Rune_PRD.md / Rune_TRD.md)

---

## 1. Overview

Runestone is a local, read-only web interface for browsing Rune repository history — commit timelines, file trees at any point in history, diffs, and branch overviews — rendered in a browser, served entirely from data already present in the local `.rune/objects` store. No network access, no GCS calls, no server deployment: it's a `localhost`-only companion to the `rune` CLI.

## 2. Problem Statement

Command-line tools (`rune log`, `rune diff`) are functional but not built for *browsing* — scanning history visually, jumping between commits, reading a diff side-by-side, or exploring a file tree at an arbitrary point in time. GitHub-style web UIs solve this well, but assume a hosted server and a remote you don't control. Since Rune already has a complete local copy of the object graph after any `fetch`/`pull`, there's no reason browsing history should require network access at all.

## 3. Goals

| Goal | Description |
|---|---|
| G1 | Provide a visual, browsable interface to local Rune repository history |
| G2 | Require zero network access — reads only from the local `.rune/objects` store |
| G3 | Ship as part of the same Rust workspace/binary philosophy as Rune (no separate language/toolchain requirement) |
| G4 | Be safe to run against untrusted/cloned repository data without becoming an attack surface itself |
| G5 | Reinforce the portfolio narrative — a polished, demoable visual layer on top of the systems-level Rune engine |

### Non-Goals (out of scope for v1)

- Any write capability (no commit, merge, or push from the UI) — strictly read-only
- Remote/GCS interaction from the UI itself (always operates on local data only)
- Multi-repository dashboard (v1 is scoped to one repo at a time, run from within it)
- Authentication/multi-user access (single local user, single machine)
- Real-time collaboration features (comments, PRs, code review) — Rune itself doesn't have a review workflow (see Rune PRD, Collaboration discussion), so Runestone has nothing to surface here in v1

## 4. Target Users

- The developer themself, browsing their own Rune repositories locally.
- Anyone demoing Rune — a visual UI is a much stronger portfolio/demo artifact than CLI output alone.

## 5. User Stories

1. As a user, I want to run one command (`rune web` or similar) and have a browser tab open showing my repo's history.
2. As a user, I want to see a commit timeline for the current branch, with author, date, and message.
3. As a user, I want to click a commit and see the full file tree as it existed at that point.
4. As a user, I want to open a file from that tree and see its contents with syntax highlighting.
5. As a user, I want to see a diff between a commit and its parent, with additions/deletions clearly marked.
6. As a user, I want to see all local and remote-tracking branches and switch which one I'm browsing.
7. As a user, I want confidence that running this tool against a cloned/pulled repository (potentially from an untrusted source) can't compromise my machine or leak data to the network.
8. As a user, I want the UI to fail gracefully (not crash the server) on corrupted or missing objects.

## 6. Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| FR1 | Local HTTP server bound to `127.0.0.1` only, on a configurable port | Must |
| FR2 | Commit history view (timeline, per branch) | Must |
| FR3 | File tree browser at any commit | Must |
| FR4 | File content viewer with syntax highlighting | Must |
| FR5 | Diff viewer (commit vs. parent; eventually arbitrary commit vs. commit) | Must |
| FR6 | Branch overview page (local + remote-tracking refs) | Must |
| FR7 | Graceful handling of binary files (no attempt to render as text) | Must |
| FR8 | Graceful handling of corrupted/missing objects (error page, not a crash) | Must |
| FR9 | Auto-open browser on launch (configurable/skippable) | Should |
| FR10 | Commit graph visualization (branch/merge topology, not just linear list) | Should |
| FR11 | Search (commits by message, files by name) | Could |
| FR12 | Blame/annotate view (line-by-line last-changed-by) | Could |
| FR13 | Dark/light theme toggle | Could |

## 7. Non-Functional Requirements

| Category | Requirement |
|---|---|
| Security | Must not be reachable from any network interface other than loopback by default; must be resilient to malicious repository content (TRD §7 covers this in detail) |
| Performance | Commit list and file tree views should render in well under 1s for repos with thousands of commits, via pagination/lazy loading rather than loading full history at once |
| Portability | Ships in the same binary/workspace as Rune — no separate install, no Node/npm toolchain required to *run* it (build-time tooling is a separate question, see TRD) |
| Reliability | A malformed object anywhere in history must not crash the whole server — errors are scoped to the page/request that hit them |
| Usability | Should feel immediately familiar to anyone who's used GitHub's commit/file/diff views |

## 8. Success Metrics (portfolio context)

- `rune web` launches a working local server and browser view against a real repository with meaningful history.
- Commit timeline, file browser, and diff viewer all function correctly against repos containing binary files, deeply nested directories, and merge commits.
- Running Runestone against the Phase 6 adversarial test corpus (from the main Rune plan) does not crash the server, expose the filesystem outside the repo, or make any outbound network call.

## 9. Pros and Cons

### Pros
- **Zero marginal infrastructure** — no server to deploy, no hosting cost; it's a view over data you already have locally.
- **Strong demo value** — turns an otherwise CLI-only portfolio project into something visually presentable in an interview or a README GIF.
- **Consistent with Rune's design philosophy** — single-binary, no-server, local-first — rather than being a bolt-on that contradicts the parent project's story.
- **Low security surface relative to a real web app** — read-only, local-only, single-user, no auth system to get wrong, no database, no write path.
- **Reuses existing Rune internals** — object decoding, diff engine, and commit graph traversal are already built for the CLI; Runestone is primarily a rendering layer on top, not new core logic.

### Cons
- **Still a genuine attack surface**, despite being "just local" — a browser tab is an active client, and if repository content is untrusted (e.g. you clone someone else's Rune repo and browse it), rendering that content safely is a real security problem, not a formality (see TRD §7).
- **Scope creep risk** — "GitHub-like UI" is an open-ended target; v1 must stay disciplined about read-only, single-repo, no-auth, or this becomes its own multi-month project.
- **Second UI paradigm to maintain** — even choosing server-rendered HTML over a JS framework, you're now maintaining templates/CSS alongside the CLI, which is more surface area than a CLI-only tool.
- **No offline value beyond what `rune log`/`rune diff` already provide** — Runestone's entire value-add is *presentation*, not new capability; worth being honest that this is a UX investment, not a functional one.
- **Local server + browser combination has a known attack class (DNS rebinding)** that CLI tools don't face at all — this is a new risk category introduced specifically by choosing "web UI," discussed in depth in the TRD.

## 10. Risks

| Risk | Impact | Likelihood | Mitigation (TRD detail) |
|---|---|---|---|
| Malicious repo content executes script in browser (stored XSS via commit messages/file content) | High | Medium | Strict output escaping/sanitization on every rendered field (TRD §7.1) |
| DNS rebinding lets a malicious website reach the local server via the user's browser | High | Low–Medium | Origin/Host header validation, bind to loopback only (TRD §7.2) |
| Huge file/diff rendering hangs or crashes the server | Medium | Medium | Size limits before rendering, pagination, streaming (TRD §7.3) |
| Path traversal via crafted file-tree request parameters | High | Low | Same canonicalization discipline as Rune's checkout logic (TRD §7.4) |
| Binary file misrendered as text, corrupting terminal/browser or leaking encoding issues | Low | Medium | Content-type sniffing, explicit binary-file handling (TRD §7.5) |

## 11. Milestones (product-level)

1. **RS-M1 — Minimal server + commit timeline**: `rune web` launches, shows commit list for current branch.
2. **RS-M2 — File tree + file viewer**: browse tree at a commit, view file contents with syntax highlighting.
3. **RS-M3 — Diff viewer**: commit-vs-parent diff rendering.
4. **RS-M4 — Branch overview + navigation polish**: branch switcher, basic commit graph view.
5. **RS-M5 — Hardening**: full security pass per TRD §7, adversarial content testing.

## 12. Open Questions

- Should Runestone ship as a subcommand (`rune web`) or a fully separate binary (`runestone`)? (Leaning subcommand, for single-binary simplicity — see TRD.)
- Is a commit-graph *visualization* (branch/merge topology, like GitHub's network graph) in scope for v1, or deferred as a "Could" item given its complexity relative to value?
- Do we want build-time asset bundling (CSS/JS embedded into the binary via `rust-embed`) from day one, to preserve the "single binary" property, or is a `static/` folder acceptable for v1?
