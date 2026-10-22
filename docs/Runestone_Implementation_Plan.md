# Implementation Plan
## Runestone — A Local Web UI for Rune

**Version:** 1.0
**Related documents:** Runestone_PRD.md, Runestone_TRD.md

---

## Assumption on pace

Same assumption as the parent Rune plan: **part-time, solo development, ~8–12 focused hours/week**. Durations compress by roughly 2.5–3x if you can work closer to full-time in bursts (noted at the end).

## Prerequisite

Runestone only depends on **Phase 2** of the main Rune implementation plan (object store + commits/refs/branches). It does **not** need Phase 3 (diff/merge — though the diff *viewer* here benefits once that lands), Phase 4/5 (remote, GCS), or Phase 6 (hardening) to be complete first. You could realistically start Runestone right after Rune's Phase 2 as a parallel track, rather than waiting for the full Rune roadmap to finish.

---

## Phase RS-0 — Workspace Setup
**Duration: 2–3 days**

- Add `rune-web` crate to the existing Cargo workspace
- Wire it in as a `rune web` subcommand of `rune-cli` (TRD §2)
- Add dependencies: `axum`, `askama`, `syntect`, `rust-embed`, `tokio` (already present for Rune's remote work — confirm feature flags cover this crate too)
- Minimal "hello world" route to confirm the server boots, binds to `127.0.0.1` only, and is reachable

**Exit criteria:** `rune web` starts a server on loopback, a browser can load a placeholder page.

---

## Phase RS-1 — Commit Timeline *(PRD RS-M1, TRD §3, §5)*
**Duration: 1–1.5 weeks**

- `GET /branch/:name/commits` route, reusing `rune-core`'s commit graph traversal (Rune TRD §3.4) — no new traversal logic
- Askama template for the timeline (author, date, message, short hash)
- Pagination (don't load entire history into one response — PRD §7 NFR)
- `GET /` redirect to current branch's timeline
- Basic CSS pass — doesn't need to be polished yet, just readable

**Exit criteria:** Can browse a real repo's commit history in the browser, paginated correctly on repos with hundreds+ of commits.

---

## Phase RS-2 — File Tree & File Viewer *(PRD RS-M2, TRD §5, §7.4, §7.5)*
**Duration: 1.5–2 weeks**

- `GET /commit/:hash/tree` and `/tree/*path` routes
- Path-safety validation on route params from day one (TRD §7.4) — don't defer this to the hardening phase, since it's cheap to do correctly now and easy to forget later
- `GET /commit/:hash/blob/*path`: content-sniffing to distinguish text/binary (TRD §7.5)
- `syntect` integration for syntax highlighting on text files
- Binary file notice UI (size, type if detectable, no render attempt)

**Exit criteria:** Can navigate the full file tree at any commit and view both text (highlighted) and binary (notice-only) files correctly.

---

## Phase RS-3 — Diff Viewer *(PRD RS-M3, TRD §3, §5)*
**Duration: 1–1.5 weeks**

- `GET /commit/:hash/diff`, reusing Rune's `similar`-based diff engine output (Rune TRD §3.2) — this phase is mostly an HTML rendering layer over existing computation
- Unified (or side-by-side, pick one for v1) diff view with add/remove highlighting
- Size ceiling before attempting to render very large diffs (TRD §7.3) — implement alongside the feature, not as a follow-up patch

**Exit criteria:** Diff view correctly renders additions/deletions for a range of real commits, including merge commits.

---

## Phase RS-4 — Branch Overview & Navigation Polish *(PRD RS-M4)*
**Duration: 1 week**

- `GET /refs`: local + remote-tracking branch listing
- Branch switcher UI, consistent navigation between timeline/tree/diff views
- `htmx`-based (or minimal vanilla JS) progressive enhancement for tree expand/collapse and paginated loading, if not already done in RS-1/RS-2
- General UI polish pass — this is the phase where "portfolio demo quality" gets dialed in

**Exit criteria:** A coherent, navigable UI — someone unfamiliar with the project could open it and understand the repo's history without CLI help.

---

## Phase RS-5 — Hardening *(PRD RS-M5, TRD §7 in full)*
**Duration: 1.5–2 weeks**

Do not compress this phase — the TRD's §7 vulnerability list is specific to the "local server + browser" combination and isn't optional polish:

- Host/Origin header validation middleware, with tests asserting rejection of forged headers (TRD §7.2 — DNS rebinding defense)
- Stored-XSS test pass: commit messages/filenames/file content containing `<script>`, HTML entities, etc. — assert everything renders escaped (TRD §7.1)
- Confirm Askama's auto-escaping is never bypassed anywhere in the codebase (grep for `|safe` or equivalent, review each usage)
- Resource-limit tests: oversized files/diffs return a graceful notice, not a hang (TRD §7.3)
- Reuse the adversarial repo corpus from Rune's own Phase 6 (path-traversal, malformed objects) against Runestone's routes — confirm no crash, no filesystem escape, no unexpected behavior (TRD §7.4, §7.6)
- `cargo audit` pass on the new dependency tree (TRD §7.8)
- Browser devtools network-tab check: confirm zero outbound requests from any served page

**Exit criteria:** Full TRD §7 checklist passes; adversarial corpus run produces clean rejections/notices with no crashes or escapes.

---

## Summary Timeline

| Phase | Focus | Part-time (8–12 hrs/wk) | If closer to full-time |
|---|---|---|---|
| RS-0 | Workspace setup | 2–3 days | 1 day |
| RS-1 | Commit timeline | 1–1.5 weeks | 2–3 days |
| RS-2 | File tree & viewer | 1.5–2 weeks | 3–4 days |
| RS-3 | Diff viewer | 1–1.5 weeks | 2–3 days |
| RS-4 | Branch overview & polish | 1 week | 2–3 days |
| RS-5 | Hardening | 1.5–2 weeks | 3–5 days |
| **Total** | | **~6.5–8.5 weeks** | **~2–3 weeks** |

## Suggested duration

**Plan around 7–8 weeks** part-time for a complete, hardened v1. Since Runestone only requires Rune's Phase 2 as a prerequisite, you have flexibility in sequencing: run it as a parallel track alongside Rune's Phase 3 onward, or slot it in as a full break after Rune's core local engine is done and before diving into diff/merge (Rune Phase 3) or the GCS remote (Rune Phase 5) — whichever keeps momentum and motivation higher.

As with the main plan: **don't compress Phase RS-5.** A local web UI with an unpatched DNS-rebinding gap or an XSS hole is a worse portfolio artifact than a CLI-only tool with no such surface at all — the security story is part of what makes this impressive, not incidental to it.

## Suggested tracking approach

Same chunk-by-chunk cadence as the main Rune plan: implement one phase, verify it against its exit criteria and the relevant TRD section, then move on — rather than planning multiple phases deep before writing code.
