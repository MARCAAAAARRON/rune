# Product Requirements Document (PRD)
## Rune — A Rust Version Control System with Google Cloud Storage Backend

**Version:** 1.0
**Status:** Draft
**Owner:** Cliny
**Related document:** Rune_TRD.md (Technical Requirements Document)

---

## 1. Overview

Rune is a Git-like distributed version control system, implemented in Rust, that uses Google Cloud Storage (GCS) as its remote backend instead of a dedicated server process. It targets developers who want Git-equivalent workflows (branch, commit, merge, push, pull) without operating remote infrastructure — the "remote" is just a cloud storage bucket.

This is a portfolio project intended to demonstrate systems-level engineering: content-addressed storage, distributed consistency, CLI tooling, and cloud integration.

## 2. Problem Statement

Git requires either a hosted service (GitHub/GitLab/Bitbucket) or a self-managed server (`git daemon`, smart HTTP backend) to support remotes. Both add operational overhead:

- Hosted services introduce third-party dependency, potential cost at scale, and data residency questions.
- Self-hosted servers require provisioning, patching, uptime management, and auth infrastructure.

There is no lightweight, serverless way to get real push/pull semantics against your own cloud storage with strong consistency guarantees.

## 3. Goals

| Goal | Description |
|---|---|
| G1 | Provide local VCS functionality equivalent to core Git commands (init, add, commit, log, branch, checkout, merge, diff) |
| G2 | Support a remote model backed directly by GCS, with no custom server required |
| G3 | Guarantee safe concurrent pushes (no silent ref clobbering) using GCS's native consistency primitives |
| G4 | Ship as a single statically-linked CLI binary |
| G5 | Serve as a demonstrable, well-documented portfolio artifact showing distributed-systems and cloud-integration competency |

### Non-Goals (out of scope for v1)

- GUI or IDE plugin integration
- Full Git protocol compatibility (Rune is Git-*like*, not a Git reimplementation/interop layer)
- Multi-cloud backend support in v1 (design should not preclude it later)
- Server-mediated access control / permission model beyond what GCS IAM provides
- Rebase, cherry-pick, bisect, submodules (candidates for v2+)

## 4. Target Users

- **Primary:** The developer themself — solo/small-team use, portfolio demonstration, learning vehicle.
- **Secondary:** Small teams or individuals who want a Git-like workflow against their own cloud storage without paying for or operating a forge.

## 5. User Stories

1. As a developer, I want to `rune init` a repository so I can start tracking file history locally.
2. As a developer, I want to `rune add` and `rune commit` so I can snapshot my working tree with a message.
3. As a developer, I want to create and switch branches so I can work on features in isolation.
4. As a developer, I want to merge branches and be shown conflicts I need to resolve manually.
5. As a developer, I want to configure a GCS bucket as `origin` so I can push my history to the cloud.
6. As a developer, I want `rune push` to fail safely (not corrupt remote state) if someone else pushed first.
7. As a developer, I want `rune pull` to fetch only the objects I'm missing, not the whole history every time.
8. As a developer, I want to see meaningful error messages when authentication to GCS fails.
9. As a team member, I want confidence that a malicious or malformed repository can't corrupt my local filesystem when I clone/pull it.

## 6. Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| FR1 | `init` creates a local repository structure | Must |
| FR2 | `add` stages file changes (content-addressed blobs) | Must |
| FR3 | `commit` creates an immutable commit object with parent linkage | Must |
| FR4 | `log` displays commit history traversal | Must |
| FR5 | `branch` / `checkout` manage and switch refs | Must |
| FR6 | `diff` shows changes between working tree, staging, and commits | Must |
| FR7 | `merge` performs 3-way merge with conflict markers on collision | Must |
| FR8 | `remote add` configures a GCS bucket + path prefix as a named remote | Must |
| FR9 | `push` uploads missing objects and atomically updates the remote ref | Must |
| FR10 | `pull` / `fetch` downloads missing objects and fast-forwards or reports divergence | Must |
| FR11 | Authentication via GCS service account credentials (env var / file path) | Must |
| FR12 | `clone` initializes a new local repo from a GCS remote | Should |
| FR13 | `.runeignore` file support | Should |
| FR14 | Commit signing (future: GPG or minisign) | Could |
| FR15 | Shallow clone / partial history fetch | Could |

## 7. Non-Functional Requirements

| Category | Requirement |
|---|---|
| Performance | Push/pull must transfer only missing objects, not full history, for repos with existing shared ancestry |
| Reliability | A failed/interrupted push must never leave the remote ref pointing at a partially-uploaded commit |
| Portability | Single binary, Linux/macOS/Windows via Rust cross-compilation |
| Security | No plaintext credential storage in the repo; validate all paths before writing to the local filesystem |
| Observability | Verbose/debug logging mode for diagnosing push/pull/auth failures |
| Usability | CLI UX and error messages should be self-explanatory without consulting docs for common errors |

## 8. Success Metrics (portfolio context — informal, not production KPIs)

- Can perform a full branch → commit → push → clone-elsewhere → pull round trip correctly.
- Two concurrent pushes to the same branch: exactly one succeeds, the other is rejected with a clear "remote has diverged" error (never silent data loss).
- Correctly detects and merges non-conflicting changes; correctly flags real conflicts.
- Survives an adversarial "evil repo" test suite (crafted objects designed to path-traverse, decompression-bomb, or otherwise misbehave) without escaping the intended write directory or exhausting resources.

## 9. Pros and Cons of the Product Approach

### Pros
- **No server to operate** — GCS itself is the only "infrastructure," which drastically lowers operational burden versus self-hosting a Git server.
- **Strong consistency by construction** — leaning on GCS's read-after-write consistency and conditional writes avoids building custom distributed-consensus machinery.
- **Portfolio value** — touches content-addressed storage, Merkle-DAG history, concurrency control, and cloud SDK integration, all in one project.
- **Cost model** — pay-for-storage/egress only, no idle compute cost for a server.
- **Portable** — a bucket is trivially copyable/mirrorable/backup-able using standard GCS tooling, independent of Rune itself.

### Cons
- **No smart protocol negotiation** — because there's no server, Rune can't do server-side computation (e.g., server-side merge, custom hooks, access control beyond IAM). Everything is client-computed, which shifts trust and correctness burden onto every client.
- **Weaker access control model** — permissions are only as granular as GCS IAM/bucket policy; no per-branch protection rules, no server-side hooks (e.g., "block force-push," "require CI pass") without building that logic into every client, which is unenforceable against a malicious client.
- **No compression/delta-negotiation server-side** — a real Git smart-HTTP server can compute a minimal packfile server-side; a dumb-remote design pushes more bytes on average for equivalent history, since the client must reason locally about what the remote has.
- **Vendor coupling** — tightly designed around GCS's specific consistency guarantees (generation preconditions); porting to another backend without those guarantees would need a different concurrency strategy.
- **Not Git-interoperable** — can't be cloned by Git clients, can't reuse Git tooling/ecosystem (diff viewers, GUI clients, CI integrations built for `.git`).
- **Solo-maintainer trust model** — with no server enforcing invariants, a buggy or compromised client can, in principle, still write malformed objects directly to the bucket (mitigated but not eliminated by client-side validation on read).

## 10. Risks

| Risk | Impact | Likelihood | Mitigation (see TRD for detail) |
|---|---|---|---|
| Concurrent push race corrupts ref | High | Medium | Atomic conditional writes (GCS generation match) |
| Malicious/malformed repo attacks local filesystem on checkout | High | Medium | Strict path canonicalization, reject `..`/absolute paths/symlinks in tree entries |
| Credential leakage (service account key) | High | Low–Medium | Never commit credentials; support workload identity / short-lived tokens |
| Large object / decompression bomb DoS | Medium | Low | Size limits before and during decompression |
| Bucket misconfigured as public | High | Low | Documentation + startup warning if bucket ACL appears public |
| Hash collision / object substitution | Low (SHA-256) | Very low | Use SHA-256, not SHA-1, from the start |

*(Full vulnerability analysis with attack scenarios lives in the TRD, Section 9.)*

## 11. Milestones (product-level)

1. **M1 — Local core**: init/add/commit/log/branch/checkout, no remote.
2. **M2 — Diff & merge**: working diff engine, 3-way merge, conflict markers.
3. **M3 — Remote abstraction + local-disk remote**: prove push/pull logic against a fake "remote" directory before touching cloud.
4. **M4 — GCS remote**: real bucket-backed push/pull/clone, auth, atomic ref updates.
5. **M5 — Hardening**: adversarial input testing, path/size validation, docs, packaging.

## 12. Open Questions

- Should `.rune/config` support multiple named remotes from day one, or just `origin` initially?
- Do we want optional local encryption-at-rest for objects before upload (client-side encryption), or rely solely on GCS's own encryption-at-rest?
- Is a lockfile-based fallback needed for any GCS API path that doesn't support conditional writes (e.g., very old client library edge cases)?
