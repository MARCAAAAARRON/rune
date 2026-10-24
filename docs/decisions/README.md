# Architecture Decision Records (ADRs)

This directory documents the significant architectural decisions made during the design and development of Rune.

## ADR Index

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-001](ADR-001-content-addressed-storage-sha256-zstd.md) | Content-Addressed Storage using SHA-256 and Zstandard Compression | Accepted | 2026-10-15 |
| [ADR-002](ADR-002-serverless-object-storage-and-cas-concurrency.md) | Serverless Remote Synchronization via GCS and Atomic CAS Preconditions | Accepted | 2026-10-16 |
| [ADR-003](ADR-003-path-traversal-defense-and-tree-sandboxing.md) | Path Traversal Defense and Strict Tree Sandboxing | Accepted | 2026-10-18 |
| [ADR-004](ADR-004-offline-read-only-web-ui-runestone.md) | Offline Read-Only Web Interface (Runestone) with Loopback Binding | Accepted | 2026-10-20 |
| [ADR-005](ADR-005-three-way-merge-lca-and-myers-diff.md) | Directed Acyclic Graph (DAG) 3-Way Merge with LCA and Myers Diffing | Accepted | 2026-10-22 |

---

## When to Record an ADR

Record an ADR whenever you make an architectural choice that:
- Chooses between competing engineering designs or libraries
- Establishes or changes data models, storage formats, or protocols
- Establishes security boundaries or threat mitigations
- Would be costly or time-consuming to reverse later

## ADR Lifecycle

```
PROPOSED ──► ACCEPTED ──► SUPERSEDED (by ADR-XXX)
                       └──► DEPRECATED
```

Never delete past ADRs; they preserve vital historical rationale and constraints. When a decision is revised, create a new ADR that explicitly references and supersedes the predecessor.

## ADR Template

```markdown
# ADR-XXX: [Title]

## Status
Proposed | Accepted | Superseded by ADR-YYY | Deprecated

## Date
YYYY-MM-DD

## Context
[Problem statement, technical requirements, security considerations, and constraints]

## Decision
[What was chosen and how it is implemented]

## Alternatives Considered

### Alternative 1
- **Pros:** ...
- **Cons:** ...
- **Rejected because:** ...

## Consequences
[Positive consequences, trade-offs, maintenance implications, and risks]
```
