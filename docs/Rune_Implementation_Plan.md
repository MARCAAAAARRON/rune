# Implementation Plan
## Rune — A Rust Version Control System with Google Cloud Storage Backend

**Version:** 1.0
**Related documents:** Rune_PRD.md, Rune_TRD.md

---

## Assumption on pace

This plan assumes **part-time, solo development** — roughly 8–12 focused hours/week, fitting around coursework (consistent with building an AI engineering portfolio alongside other active projects). If you can go closer to full-time on this for stretches, durations below compress by roughly 2.5–3x — I've noted both at the end.

Each phase below maps directly to a milestone from the PRD (§11) and a set of sections from the TRD, so you always know which document to reference while implementing.

---

## Phase 0 — Project Setup
**Duration: 2–3 days**

- Initialize Cargo workspace, choose crate structure (e.g. `rune-core`, `rune-cli` as separate crates from the start, so the engine stays decoupled from the CLI)
- Add core dependencies: `sha2`, `zstd`, `clap`, `thiserror`, `anyhow`
- Set up CI (GitHub Actions): build + test on push, across at least Linux (add macOS/Windows later)
- Establish test harness pattern (temp-dir based repo fixtures) early — you'll reuse it in every phase

**Exit criteria:** `cargo build` and `cargo test` run clean in CI on an empty skeleton.

---

## Phase 1 — Core Object Store *(PRD Milestone M1, TRD §2–3.1)*
**Duration: 1.5–2 weeks**

- Implement blob/tree/commit object types and their binary encoding (TRD §2.1)
- SHA-256 content addressing, zstd compression
- Local disk storage with sharded `objects/<ab>/<cdef...>` layout (TRD §2.2)
- `init` command: creates `.rune/` structure
- Unit tests: encode/decode round-trip, hash stability, dedup behavior (identical content → identical object)

**Exit criteria:** Can hash and store arbitrary blobs/trees/commits locally and read them back byte-identical.

---

## Phase 2 — Staging, Commits, Refs, Branches *(PRD M1 cont'd, TRD §2.2, §3.4)*
**Duration: 1.5–2 weeks**

- Index (staging area) format and `add` command
- `commit` command: builds tree from index, links parent commit(s), writes ref
- `refs/heads/<branch>`, `HEAD`, `branch`, `checkout` (working-tree writer — apply the path-safety rules from TRD §9.1/§9.3 **now**, not later, since checkout logic gets reused by clone/pull in Phase 5)
- `log`: commit graph traversal with visited-set (TRD §3.4, §9.5)
- Integration tests: init → add → commit → branch → checkout → log, full round trip

**Exit criteria:** M1 from the PRD is functionally complete — a usable local VCS with no remote.

---

## Phase 3 — Diff & Merge *(PRD M2, TRD §3.2–3.3)*
**Duration: 2–2.5 weeks**

This is usually the trickiest phase — budget the extra time here rather than compressing it.

- Tree diff (recursive, short-circuit on matching subtree hashes)
- File-level diff via `similar` crate (Myers algorithm)
- `diff` command output formatting
- Merge base / lowest-common-ancestor search (BFS over commit DAG)
- 3-way merge logic + conflict marker generation
- Property-based tests (`proptest`) for diff/merge edge cases: empty files, binary files, identical content, deep nesting (TRD §7)

**Exit criteria:** Can merge two diverged branches, auto-resolving non-conflicting changes and correctly flagging real conflicts with markers.

---

## Phase 4 — Remote Abstraction + Local Remote *(PRD M3, TRD §4.1)*
**Duration: 1 week**

- Define the `Remote` trait (TRD §4.1)
- Implement `LocalRemote` (just another directory) purely for testing push/pull logic without cloud dependencies yet
- Implement push algorithm (TRD §4.2) and pull/fetch algorithm (TRD §4.4) against `LocalRemote`
- Concurrency test: simulate two "clients" racing a push against the same `LocalRemote` branch, assert exactly one wins (TRD §7)

**Exit criteria:** Push/pull/clone work correctly end-to-end against a fake local remote — this de-risks the GCS integration in Phase 5, since the protocol logic is already proven.

---

## Phase 5 — GCS Remote *(PRD M4, TRD §4.3, §6)*
**Duration: 1.5–2 weeks**

- Integrate `object_store` crate's GCS backend
- Implement `GcsRemote` against the same `Remote` trait
- Wire up conditional writes (`ifGenerationMatch`) for atomic ref updates (TRD §4.3) — this is the one piece of genuinely new logic versus `LocalRemote`, everything else in the trait implementation is mostly plumbing
- `remote add` command, credential handling via GCP Application Default Credentials (TRD §9.7)
- Bounded-concurrency object transfer (`buffer_unordered`, TRD §6)
- Re-run the Phase 4 concurrency test, this time against a real GCS test bucket

**Exit criteria:** PRD's core success metric — full branch → commit → push → clone-elsewhere → pull round trip against a real bucket, with concurrent-push safety verified.

---

## Phase 6 — Hardening *(PRD M5, TRD §9 in full)*
**Duration: 1.5–2 weeks**

Treat this as its own phase, not cleanup time squeezed into the others — the TRD's §9 vulnerability list is long enough to warrant dedicated attention:

- Build the adversarial test corpus: path-traversal tree entries, symlink-escape attempts, oversized/decompression-bomb blobs, cyclic commit graphs, malformed length prefixes (TRD §9.1–9.6)
- Run the full corpus through fetch/checkout, assert clean rejection with no filesystem escape or resource exhaustion
- Credential-log redaction pass; document least-privilege IAM setup (TRD §9.7)
- `.runeignore` support (PRD FR13)
- Error message pass — make failure modes self-explanatory (PRD §7, Usability)
- Cross-platform build via `cross`; packaging for GitHub Releases

**Exit criteria:** Adversarial test suite passes in full; binary builds and runs on Linux/macOS/Windows.

---

## Summary Timeline

| Phase | Focus | Part-time (8–12 hrs/wk) | If closer to full-time |
|---|---|---|---|
| 0 | Setup | 2–3 days | 1 day |
| 1 | Object store | 1.5–2 weeks | 3–4 days |
| 2 | Staging/commits/refs | 1.5–2 weeks | 3–4 days |
| 3 | Diff & merge | 2–2.5 weeks | 5–7 days |
| 4 | Remote abstraction + local remote | 1 week | 2–3 days |
| 5 | GCS remote | 1.5–2 weeks | 3–5 days |
| 6 | Hardening | 1.5–2 weeks | 3–5 days |
| **Total** | | **~10–13 weeks** | **~3–4 weeks** |

## Suggested duration

**Plan around 11–12 weeks** for the full v1 (through PRD Milestone M5) at a sustainable part-time pace. Phase 3 (diff/merge) and Phase 6 (hardening) are the two most likely to run long — they're also the two that most directly demonstrate engineering rigor if you're using this for a portfolio, so that's time well spent rather than schedule slippage to worry about.

If you want a faster path to something demoable, Phases 0–2 (~3–4 weeks part-time) already get you a working local VCS you can show off, even before the GCS remote exists — worth keeping in mind if you need an earlier checkpoint for a resume/portfolio deadline.

## Suggested tracking approach

Given you prefer incremental, chunk-by-chunk execution: I'd suggest we treat each phase above as its own work session (or a few sessions) — implement, test, review the rationale against the TRD, then move to the next phase, rather than planning further ahead than one phase at a time in detail.
