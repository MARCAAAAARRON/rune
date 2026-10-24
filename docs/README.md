# Rune Documentation Map

Welcome to the documentation suite for **Rune** (the distributed VCS) and **Runestone** (the local web inspection UI).

---

## 1. User Guides & Manuals

- [Rune CLI User Manual](rune_cli_manual.md): Comprehensive guide to using the `rune-cli` command line tool, everyday workflows (init, add, commit, branch, checkout, merge, diff), and remote synchronization (GCS and local directories).
- [Runestone Web Manual](runestone_web_manual.md): Guide to starting and navigating the local read-only web interface (`rune-cli web`), exploring timelines, inspecting file trees, viewing syntax-highlighted code, and analyzing diffs.

---

## 2. Architecture & Decision Records

- [Architecture Decision Records (ADRs)](decisions/README.md): Index of high-level technical decisions, rationale, trade-offs, and alternatives considered.
  - [ADR-001: Content-Addressed Storage (SHA-256 & Zstandard)](decisions/ADR-001-content-addressed-storage-sha256-zstd.md)
  - [ADR-002: Serverless Remote Synchronization & CAS Concurrency](decisions/ADR-002-serverless-object-storage-and-cas-concurrency.md)
  - [ADR-003: Path Traversal Defense & Strict Tree Sandboxing](decisions/ADR-003-path-traversal-defense-and-tree-sandboxing.md)
  - [ADR-004: Offline Read-Only Web UI (Runestone) & Loopback Security](decisions/ADR-004-offline-read-only-web-ui-runestone.md)
  - [ADR-005: DAG 3-Way Merge with LCA & Myers Diffing](decisions/ADR-005-three-way-merge-lca-and-myers-diff.md)

---

## 3. Technical & Product Specifications

- [Rune Technical Requirements (TRD)](Rune_TRD.md): In-depth system design, Merkle DAG data structures, binary encoding, remote protocols, and threat modeling for Rune VCS.
- [Rune Product Requirements (PRD)](Rune_PRD.md): Product scope, user personas, functional specifications, non-functional requirements, and project milestones.
- [Runestone Technical Requirements (TRD)](Runestone_TRD.md): Architecture of the embedded web UI, Axum route hierarchy, template rendering, and security middleware.
- [Runestone Product Requirements (PRD)](Runestone_PRD.md): User experience goals, views, and read-only offline principles for the web UI.

---

## 4. Implementation Roadmaps

- [Rune Implementation Plan](Rune_Implementation_Plan.md): Milestone breakdown and engineering phases from core storage engine to remote cloud synchronization.
- [Runestone Implementation Plan](Runestone_Implementation_Plan.md): Phased roadmap for web routes, commit timelines, tree views, and diff rendering.
