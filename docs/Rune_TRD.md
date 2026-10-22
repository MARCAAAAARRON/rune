# Technical Requirements Document (TRD)
## Rune — A Rust Version Control System with Google Cloud Storage Backend

**Version:** 1.0
**Status:** Draft
**Owner:** Cliny
**Related document:** Rune_PRD.md (Product Requirements Document)

---

## 1. Architecture Overview

Rune follows Git's core model: an immutable, content-addressed object graph (a Merkle DAG), plus mutable references (branches/HEAD) that point into it. The system is split into independent layers so each is testable in isolation:

```
┌─────────────────────────────────────────────┐
│  CLI (clap)                                  │
├─────────────────────────────────────────────┤
│  Porcelain commands (commit, merge, push...) │
├─────────────────────────────────────────────┤
│  Core engine                                 │
│   - Object store (blob/tree/commit)          │
│   - Index / staging area                     │
│   - Refs (branches, HEAD)                    │
│   - Diff / merge algorithms                  │
├─────────────────────────────────────────────┤
│  Remote trait (abstraction)                  │
│   - LocalRemote (dev/test)                   │
│   - GcsRemote   (production)                 │
├─────────────────────────────────────────────┤
│  Storage backends                            │
│   - Local filesystem (.rune/objects)         │
│   - object_store crate → GCS                 │
└─────────────────────────────────────────────┘
```

## 2. Data Model

### 2.1 Object types (content-addressed, hashed with SHA-256)

- **Blob**: raw file content, compressed (zstd) before storage.
- **Tree**: ordered list of `(name, mode, object_type, hash)` entries representing a directory snapshot.
- **Commit**: `{ tree_hash, parent_hashes[], author, committer, timestamp, message }`.

Object ID = `SHA256(object_type || length || content)` — same construction pattern as Git, but SHA-256 instead of SHA-1 to avoid known collision-attack classes affecting SHA-1.

### 2.2 On-disk / on-bucket layout

```
objects/<hash[0:2]>/<hash[2:]>      # sharded to avoid huge single directories
refs/heads/<branch-name>            # file containing a commit hash
refs/remotes/<remote>/<branch>      # last-known remote state (local cache)
HEAD                                 # "ref: refs/heads/main" or a detached hash
config                               # remote definitions, user identity
index                                # staging area (serialized)
```

The same `objects/` and `refs/` layout is used both locally (`.rune/`) and remotely (bucket prefix), which is what makes push/pull a straightforward set-diff rather than a bespoke protocol.

## 3. Core Algorithms

### 3.1 Content addressing & storage
- Hash function: SHA-256.
- Object encoding: type tag + length-prefixed body (varint length, bounds-checked — see §9.6), then zstd-compressed.
- Deduplication is automatic: identical content across files/commits hashes to the same object and is stored once.

### 3.2 Diff
- Tree diff: recursive comparison of two tree objects by entry name; O(entries) per level, not full file rescans, when trees share subtrees (unchanged subtrees short-circuit by hash equality).
- File diff: Myers diff algorithm (or the `similar` crate, which implements it) for line-level text diffs.

### 3.3 Merge
- 3-way merge using the merge base (lowest common ancestor in the commit DAG, found via BFS/DFS walk with visited-set).
- Per-file: if only one side changed a hunk, take it; if both sides changed the same region, emit conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) and mark the file as unresolved in the index.
- Tree-level: files added/deleted on only one side auto-resolve; concurrent add-with-different-content at the same path is a conflict.

### 3.4 Commit graph traversal
- `log`: DFS/BFS from HEAD following parent pointers, with a visited-set to handle merge commits (multiple parents) without repeat traversal or infinite loops on malformed cyclic input (see §9 — a cycle would be invalid/adversarial input and must be detected, not trusted).

## 4. Remote Protocol Design

### 4.1 Remote trait

```rust
#[async_trait]
trait Remote {
    async fn list_refs(&self) -> Result<HashMap<String, ObjectId>>;
    async fn has_object(&self, id: &ObjectId) -> Result<bool>;
    async fn fetch_object(&self, id: &ObjectId) -> Result<Vec<u8>>;
    async fn upload_object(&self, id: &ObjectId, data: &[u8]) -> Result<()>;
    async fn update_ref(&self, name: &str, expected_old: Option<ObjectId>, new: ObjectId) -> Result<()>;
}
```

`update_ref` takes an *expected old value* — this is the hook into GCS's conditional-write mechanism (§4.3).

### 4.2 Push algorithm

1. Read local ref for the branch being pushed (e.g. `refs/heads/main` → commit `C`).
2. Fetch remote's current value for that ref (or `None` if it doesn't exist yet).
3. If remote ref exists and is **not** an ancestor of `C` in the local history → reject locally as "non-fast-forward" (mirrors Git's default safety behavior) unless `--force` is explicitly passed.
4. Walk the commit graph from `C` backward, stopping at any commit hash the remote already reports having (via `list_refs`/`has_object` checks) — this yields the minimal missing object set.
5. Upload missing objects (blobs, trees, commits) — order doesn't strictly matter for correctness since objects are content-addressed and immutable, but uploading blobs/trees before the commit that references them avoids a brief window where the ref points at an incomplete object set if a reader races in.
6. Call `update_ref(branch, expected_old = remote's_last_known_C, new = C)`.
7. If the conditional update fails (someone else pushed in the meantime), abort with a "remote has diverged, pull first" error. **Never retry-and-overwrite automatically.**

### 4.3 Atomicity via GCS generation preconditions

GCS objects carry a monotonically increasing **generation number** on every write. The `ifGenerationMatch` precondition lets a write be conditioned on "only succeed if the object's generation is still exactly N" (or "doesn't exist yet," via `ifGenerationMatch=0`). Rune stores each ref as a small object (`refs/heads/main`) and uses this precondition for every ref update. This is the entire concurrency-control mechanism — no distributed lock service, no server, no polling — and it degrades safely: a losing writer gets an HTTP 412 Precondition Failed, which Rune surfaces as "diverged, pull first."

### 4.4 Pull/fetch algorithm

1. `list_refs` on remote to get its current branch heads.
2. Walk backward from the remote commit hash, stopping at any hash already present locally, to compute the missing object set.
3. Download missing objects, verify each one's hash on arrival (recompute SHA-256 of decompressed content, compare to the object ID used to request it — reject on mismatch, see §9.2).
4. Update `refs/remotes/<remote>/<branch>` to the new value.
5. `merge`/fast-forward local branch as a separate, explicit step (mirrors Git's fetch/merge split — pull never silently rewrites local branch state without the fast-forward or merge logic running).

## 5. Technology Stack

| Concern | Choice | Rationale |
|---|---|---|
| Language | Rust (2021 edition) | Memory safety for a tool that parses untrusted input (cloned repos); performance for hashing/diffing |
| CLI | `clap` | De facto standard, derive macros keep command definitions declarative |
| Hashing | `sha2` crate | Well-audited SHA-256 implementation |
| Compression | `zstd` crate | Better ratio/speed tradeoff than zlib/deflate that Git traditionally uses |
| Diff | `similar` crate | Mature Myers-diff implementation |
| Cloud storage | `object_store` crate (Apache Arrow project) | Backend-agnostic (GCS today, S3/Azure later without a rewrite); actively maintained; async-native |
| Async runtime | `tokio` | Required by `object_store` and for concurrent object upload/download |
| Serialization | `serde` + a compact binary format (`bincode` or custom) for index/config; custom binary layout for objects (Git-style) for cross-tool clarity |
| Error handling | `thiserror` (library errors) + `anyhow` (CLI-level context) | Standard Rust practice for clear error propagation |

## 6. Concurrency Model

- Object uploads/downloads during push/pull are parallelized (bounded concurrency, e.g. `futures::stream::buffer_unordered(N)`) since objects are independent and immutable — safe to fetch/upload many at once.
- Ref updates are **never** parallel writes to the same ref; only one conditional update per push attempt, no retries-with-overwrite.
- Local working-tree operations (checkout, add) take an exclusive local lock file (`.rune/index.lock`, Git-style) to prevent two local Rune processes from corrupting the index concurrently.

## 7. Testing Strategy

- **Unit tests** per layer (object encoding/decoding, tree diff, merge logic) with property-based testing (`proptest`) for the diff/merge algorithms specifically, since edge cases (empty files, binary files, identical content, deeply nested trees) are easy to under-cover by hand.
- **Integration tests**: full CLI workflows via a test harness driving real temp-directory repos (init → commit → branch → merge → push-to-local-remote → clone elsewhere → pull).
- **Adversarial/security test suite** (see §9 for the specific attack classes this must cover): a corpus of deliberately malformed objects and trees, run through fetch/checkout, asserting no filesystem escape, no unbounded resource use, and a clean rejection error.
- **Concurrency tests**: simulate two clients racing a push against the same branch (against `LocalRemote` and, in CI, against a real GCS test bucket) and assert exactly one wins.

## 8. Deployment & Packaging

- Single static binary per platform (`cross` for cross-compilation: Linux x86_64/arm64, macOS, Windows).
- No runtime dependencies beyond libc; credentials supplied via standard GCS auth mechanisms (service account JSON path, or Application Default Credentials / workload identity when running in GCP).
- Distributed via GitHub Releases initially; a Cargo crate registry publish is a reasonable stretch goal.

## 9. Security & Vulnerability Analysis

This section enumerates concrete attack classes Rune must defend against, since it — like Git itself — processes data that may come from an untrusted or semi-trusted source (a cloned/pulled repository).

### 9.1 Path traversal via crafted tree entries
**Threat:** A malicious tree object contains an entry named `../../../../etc/passwd` or an absolute path. Naively joining this with the working-directory path during checkout writes outside the repo.
**Mitigation:** Canonicalize every tree entry name before writing; reject any entry containing `..` path components, path separators where a single filename is expected, absolute paths, or null bytes. This mirrors a real class of historical Git vulnerabilities (e.g. CVE-2017-1000117-adjacent path-handling issues and various `..`-in-tree CVEs across VCS tools) — it is not a hypothetical.

### 9.2 Object substitution / hash mismatch
**Threat:** A remote (compromised bucket, MITM if TLS were ever bypassed, or a buggy client) returns bytes that don't match the requested hash.
**Mitigation:** Always recompute the hash of fetched content locally and compare against the requested object ID before trusting or storing it. Never trust the remote's own claim about an object's identity.

### 9.3 Symlink attacks during checkout
**Threat:** A tree contains a symlink entry pointing outside the repo (e.g. `evil -> /etc`), and a subsequent tree in the same or a later commit writes a file "through" that symlink path, escaping the working directory.
**Mitigation:** When checking out, refuse to follow symlinks recorded in the tree as traversal targets for further writes; write the symlink itself as a symlink (matching Git's mode-based handling) but validate that no other entry's path resolves through it before writing.

### 9.4 Decompression / resource-exhaustion bombs
**Threat:** A crafted blob with a tiny compressed size but an enormous decompressed size (a zip/zstd bomb), or a tree with an absurd number of entries, aimed at exhausting memory or disk during fetch/checkout.
**Mitigation:** Enforce a maximum decompressed size read from a length header before allocating buffers, and abort decompression if actual output exceeds the declared size; cap tree entry counts and nesting depth to sane maxima; stream rather than fully buffer where feasible.

### 9.5 Commit graph cycles / malformed DAG
**Threat:** A malicious commit object lists a parent that (directly or transitively) points back to itself, turning traversal (`log`, merge-base search) into an infinite loop — a DoS.
**Mitigation:** All graph walks use a visited-set and treat a re-visited node as "already processed, stop," never as an error that retries; additionally validate acyclicity is unnecessary to prove upfront because visited-set traversal is inherently cycle-safe — but this must be enforced in every traversal site, not just the "happy path" ones (log, merge-base, and push's missing-object walk all need it independently).

### 9.6 Integer overflow / malformed length prefixes
**Threat:** A crafted object header claims a length that overflows a fixed-width integer during parsing, or is inconsistent with actual content length, leading to out-of-bounds reads or panics.
**Mitigation:** Use checked/saturating arithmetic for all length calculations; validate declared length against actual available bytes before allocating; Rust's panic-on-overflow in debug builds catches some of this in testing, but production code must handle it as a recoverable `Result`, not rely on a panic.

### 9.7 Credential handling
**Threat:** GCS service account keys committed to a repo, logged in error output, or left in shell history; overly broad IAM permissions on the bucket (e.g. bucket-wide `Storage Admin` when `Object Admin` on a prefix would do).
**Mitigation:** Never read credentials from within the repo tree; support GCP Application Default Credentials / workload identity as the preferred path over long-lived JSON keys; redact any credential-shaped strings from debug/verbose logs; document least-privilege IAM role recommendations (object create/get/list on the specific prefix, not project-wide roles).

### 9.8 Bucket misconfiguration (public exposure)
**Threat:** The GCS bucket used as a remote is accidentally made publicly readable/writable, exposing (or allowing tampering with) repository history.
**Mitigation:** Rune can't fully prevent user misconfiguration of their own cloud resources, but should: warn at `remote add` time if it can detect the bucket has public access (where permissions allow checking this), and document the recommended bucket IAM setup prominently. Note this risk is fundamentally different from Git's model — there's no equivalent "public bucket" failure mode when using GitHub, since access control is mediated by the platform rather than raw storage ACLs.

### 9.9 Lack of commit authenticity / signing (v1 gap)
**Threat:** Nothing in v1 cryptographically proves a commit's claimed author actually authored it — anyone with write access to the bucket can push a commit claiming any author identity, and there is no server-side identity check (unlike, say, a GitHub push authenticated to a specific account).
**Mitigation (future work, not v1):** Support commit signing (minisign or GPG) as in §PRD FR14; until implemented, this is a known, documented gap rather than a silently assumed guarantee — Rune's threat model for v1 should explicitly state that branch integrity relies on bucket-level access control (who can write to the bucket), not on cryptographic authorship proof.

### 9.10 Denial of service via push flooding
**Threat:** Since there's no server enforcing rate limits or quotas, a client (malicious or buggy) with write access could push large volumes of objects, driving up storage cost and request volume.
**Mitigation:** This is a genuine structural limitation of the serverless-remote design (see PRD §9, Cons) — mitigations are limited to GCS-side controls (bucket quotas, Cloud Monitoring alerts on request volume/cost) rather than anything Rune itself can enforce client-side, since a malicious client can simply not run Rune's own checks.

## 10. Pros and Cons — Technical Trade-offs

### Pros
- Content-addressed, immutable object model gives strong integrity guarantees "for free" — corruption or tampering of an object is detectable by hash mismatch (§9.2) rather than needing separate checksums.
- `object_store` abstraction means the entire remote backend is swappable; the GCS-specific code is isolated to one implementation of one trait.
- SHA-256 from day one avoids the collision-attack legacy that Git (SHA-1-based) has had to retrofit mitigations for.
- Atomic ref updates via GCS preconditions give real correctness guarantees without implementing distributed consensus.

### Cons
- All integrity/security enforcement is client-side (§9.9, §9.10) — a fundamental consequence of the no-server design, not a bug to be fixed later. This should be stated plainly in any documentation/demo of the project rather than glossed over.
- Rust's async ecosystem (tokio + object_store + async_trait) adds real complexity and compile-time cost versus a synchronous design; justified here by needing concurrent object transfer, but worth noting as a deliberate trade-off.
- No delta compression between similar-but-not-identical blobs (Git's packfile delta encoding) is in scope for v1 — every changed file's full content (post zstd-compression) is transferred, which is simpler but less bandwidth-efficient than Git's approach for large repos with many small incremental changes to big files.

## 11. Appendix — Command-to-Layer Mapping

| CLI command | Primary layer(s) touched |
|---|---|
| `init` | Object store, refs |
| `add` | Index |
| `commit` | Object store, index, refs |
| `log` | Refs, commit graph traversal |
| `branch` / `checkout` | Refs, working tree writer (§9.1, §9.3 apply here) |
| `diff` | Diff engine |
| `merge` | Diff engine, merge algorithm, index |
| `remote add` | Config, remote trait wiring |
| `push` | Remote trait, GcsRemote, conditional ref update |
| `pull` / `fetch` | Remote trait, GcsRemote, object verification (§9.2) |
| `clone` | Remote trait (fetch-all), working tree writer |
