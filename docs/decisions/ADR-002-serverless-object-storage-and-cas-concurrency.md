# ADR-002: Serverless Remote Synchronization via GCS and Atomic CAS Preconditions

## Status
Accepted

## Date
2026-10-16

## Context
Traditional distributed version control systems (like Git) typically rely on stateful servers running specialized daemons (e.g., `git-receive-pack` over SSH or Smart HTTP). Operating dedicated servers adds administrative burden, hosting costs, security patch overhead, and single points of failure.

Rune's primary design goal is serverless collaboration: allowing teams to synchronize repositories directly against cloud object stores (like Google Cloud Storage) or shared directories (like Google Drive for Desktop) without running any server process.

However, multiple developers pushing to the same remote branch concurrently introduces race conditions. If developer Alice and developer Bob push simultaneously, Bob might inadvertently overwrite Alice's branch ref without incorporating her commits (a "blind write" or split-brain issue).

## Decision
We implemented a serverless remote synchronization protocol based on the `Remote` abstraction in `rune-core::remote`, backed by Compare-and-Swap (CAS) primitives:
1. **Identical Remote Layout:** Remote endpoints store the exact same layout as the local repository: `objects/<ab>/<cdef...>` and `refs/heads/<branch>`.
2. **Atomic Ref Updates via GCS Generation Preconditions:**
   - In Google Cloud Storage, updating `refs/heads/<branch>` uses the `ifGenerationMatch` precondition header.
   - If Alice and Bob attempt to push at the same moment, only the client matching the target object's expected generation succeeds; the second client receives a generation mismatch error and must fetch/rebase before retrying.
3. **Atomic Local CAS:**
   - For directory-based remotes (`LocalRemote`), ref writes are made atomic via temporary file creation followed by an atomic filesystem rename, with pre-update content comparison.
4. **Validation Prior to Ingestion:**
   - During `fetch`, remote objects are streamed and verified against their declared SHA-256 hash *before* being decoded or written to local storage.

## Alternatives Considered

### Dedicated Remote Daemon (Custom TCP / HTTP server)
- **Pros:** Full server-side control over hooks, access control, and transaction locking.
- **Cons:** Defeats the zero-infrastructure goal; users must deploy, monitor, and pay for virtual machines or container instances.
- **Rejected because:** Direct cloud object storage eliminates maintenance overhead and delivers high availability at minimal storage cost.

### Distributed Lock Files (e.g., `refs/heads/main.lock`)
- **Pros:** Conceptually simple to understand.
- **Cons:** Prone to orphaned locks if a client crashes mid-push or loses network connectivity, requiring manual administrative intervention to release locks.
- **Rejected because:** Native object storage preconditions (`ifGenerationMatch`) provide true atomic compare-and-swap without lock file abandonment risks.

## Consequences
- **Zero Server Maintenance:** Users can create a private bucket on Google Cloud Storage and immediately push/pull code without configuring web servers or SSH keys.
- **Concurrency Safety:** Race conditions during concurrent branch updates are deterministically rejected with clear error messages.
- **Network Efficiency:** Push and pull calculate missing objects via simple set difference between local and remote ref commit DAGs, streaming only the missing immutable objects.
