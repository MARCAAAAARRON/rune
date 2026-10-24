# Rune VCS

[![CI](https://github.com/MARCAAAAARRON/rune/actions/workflows/ci.yml/badge.svg)](https://github.com/MARCAAAAARRON/rune/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Rune is a distributed Version Control System (VCS) written in Rust. Inspired by Git's content-addressable Merkle DAG model, Rune is architected specifically for memory safety, strict path sandboxing, and direct serverless cloud object storage—eliminating the need to deploy or maintain dedicated server infrastructure.

> [!NOTE]
> Rune is a standalone VCS with its own content-addressable storage format. It is **not** interoperable with standard Git repositories, remotes, or tooling.

---

## Key Features

- **Content-Addressable Storage (SHA-256):** Every commit, tree, and blob is hashed using SHA-256. Object integrity is cryptographically validated upon decompression before any parsing occurs.
- **Transparent Compression:** Objects are compressed locally and remotely using Zstandard (`zstd`) for fast read/write speeds and high compression ratios.
- **Serverless Remotes:** Supports direct synchronization with Google Cloud Storage (`gcs://<bucket>`) or local directories (including cloud mounts such as Google Drive for Desktop) without running a remote server daemon.
- **Race-Condition-Resistant (Atomic CAS):** Ref updates rely on Compare-and-Swap (CAS) with GCS generation preconditions or atomic filesystem operations, preventing concurrent push conflicts.
- **Path Traversal Defense:** Stored tree entries strictly forbid path separators (`/`, `\`) and traversal symbols (`.` and `..`), preventing malicious checkout attacks.
- **Runestone Local Web UI:** Embedded read-only web interface (`rune-cli web`) for inspecting commit histories, file trees, and diffs locally via your browser.

---

## Quick Start

### Prerequisites

- [Rust toolchain](https://www.rust-lang.org/) (edition 2021, Rust 1.70+)
- (Optional) [Google Cloud CLI](https://cloud.google.com/sdk) if authenticating against GCS buckets (`gcloud auth application-default login`)

### 1. Build and Install

```bash
# Clone the repository
git clone https://github.com/MARCAAAAARRON/rune.git
cd rune

# Build release binaries
cargo build --release

# Install rune-cli to your local cargo bin path
cargo install --path rune-cli
```

### 2. Basic Workflow

```bash
# 1. Initialize a repository in your current directory
rune-cli init

# 2. Stage files (supports .runeignore)
rune-cli add src/main.rs
rune-cli add Cargo.toml

# 3. Commit changes
rune-cli commit -m "Initial commit" --author "Developer <dev@example.com>"

# 4. Create and switch to a branch
rune-cli branch feature-auth
rune-cli checkout feature-auth

# 5. Inspect commit log and diffs
rune-cli log
rune-cli diff <old-commit-hash> <new-commit-hash>

# 6. Merge branches with 3-way LCA
rune-cli checkout main
rune-cli merge feature-auth
```

---

## Command Reference

| Command | Syntax | Description |
|---|---|---|
| `init` | `rune-cli init` | Initializes a new `.rune` repository in the current directory. |
| `add` | `rune-cli add <path>` | Stages a file or directory into the working index. |
| `commit` | `rune-cli commit -m "<msg>" [--author "<name <email>>"]` | Commits staged changes as a snapshot. |
| `log` | `rune-cli log` | Displays chronological commit history for the current branch. |
| `branch` | `rune-cli branch [name] [-d/--delete]` | Lists all branches, creates a new branch, or deletes an existing branch. |
| `checkout` | `rune-cli checkout <target>` | Switches working tree and HEAD to a branch or commit hash. |
| `diff` | `rune-cli diff <old-commit> <new-commit>` | Displays file-level and line-by-line diffs between two commits. |
| `merge` | `rune-cli merge <branch>` | Merges specified branch into the current HEAD using 3-way LCA. |
| `remote` | `rune-cli remote add <name> <url>` | Registers a remote repository alias (e.g., `origin` to a GCS bucket or directory). |
| `remote` | `rune-cli remote list` | Lists all configured remote aliases and target endpoints. |
| `push` | `rune-cli push <remote> <branch>` | Pushes branch refs and missing objects to the remote endpoint. |
| `fetch` | `rune-cli fetch <remote> <branch>` | Downloads remote objects and updates tracking refs without merging. |
| `pull` | `rune-cli pull <remote> <branch>` | Fetches objects from the remote and merges them into the current branch. |
| `web` | `rune-cli web [--port 8080]` | Launches the Runestone local read-only web UI on `127.0.0.1`. |

---

## Remotes & Synchronization

Rune supports two primary remote types:

### 1. Google Cloud Storage (`gcs://`)
Ideal for serverless team collaboration. No central Git server daemon is required:
```bash
# Set your GCS bucket as origin
rune-cli remote add origin gcs://my-team-rune-bucket

# Push your branch (uses generation preconditions to prevent write race conditions)
rune-cli push origin main
```
*Note: Make sure application credentials are configured via `gcloud auth application-default login` or the `GOOGLE_APPLICATION_CREDENTIALS` environment variable.*

### 2. Local Filesystem / Cloud Mounts
Push and pull directly from a local path, external drive, or synchronized cloud folder (such as Google Drive for Desktop):
```bash
# Add a synced folder as remote
rune-cli remote add origin "G:\My Drive\rune-backups"

# Push and fetch
rune-cli push origin main
rune-cli pull origin main
```

---

## Runestone Web Interface

Runestone provides an offline, zero-network, read-only browser UI to inspect repository history, browse trees, and inspect unified diffs:

```bash
rune-cli web --port 8080
```
Then navigate to `http://127.0.0.1:8080` in your web browser. Runestone binds exclusively to `127.0.0.1` and incorporates Host/Origin validation middleware to enforce local security boundaries and defeat DNS rebinding attacks.

---

## Architecture & Design Decisions

The Rune workspace is structured into three crates:

```
RUNE/
├── rune-core/       # Core VCS domain logic, storage engine, protocols, and algorithms
├── rune-cli/        # Command-line interface and subcommand dispatcher
└── rune-web/        # Runestone web server (Axum-based read-only browser UI)
```

### Architecture Decision Records (ADRs)

Key architectural choices and constraints are documented as ADRs in [`docs/decisions/`](docs/decisions/README.md):

- [**ADR-001**](docs/decisions/ADR-001-content-addressed-storage-sha256-zstd.md): Content-Addressed Storage using SHA-256 and Zstandard Compression
- [**ADR-002**](docs/decisions/ADR-002-serverless-object-storage-and-cas-concurrency.md): Serverless Remote Synchronization via GCS and Atomic CAS Preconditions
- [**ADR-003**](docs/decisions/ADR-003-path-traversal-defense-and-tree-sandboxing.md): Path Traversal Defense and Strict Tree Sandboxing
- [**ADR-004**](docs/decisions/ADR-004-offline-read-only-web-ui-runestone.md): Offline Read-Only Web Interface (Runestone) with Loopback Binding
- [**ADR-005**](docs/decisions/ADR-005-three-way-merge-lca-and-myers-diff.md): Directed Acyclic Graph (DAG) 3-Way Merge with LCA and Myers Diffing

For technical and product requirements, see the full documentation index in [`docs/README.md`](docs/README.md):
- [RUNE User Manual](docs/rune_cli_manual.md)
- [Runestone Web Manual](docs/runestone_web_manual.md)
- [Rune Technical Requirements (TRD)](docs/Rune_TRD.md)
- [Rune Product Requirements (PRD)](docs/Rune_PRD.md)

---

## Development & Testing

Run the full workspace test suite (including unit tests, adversarial security tests, property tests, and remote concurrency tests):

```bash
# Run all workspace tests
cargo test --workspace

# Run tests with strict compiler and linter checks
cargo clippy --workspace --all-targets

# Check code formatting
cargo fmt --all -- --check
```

### Test Suite Breakdown

- **`rune-core` integration tests:** Verify full end-to-end commit, branch, checkout, and ignore flows.
- **`rune-core` adversarial tests:** Verify path traversal rejection, symlink escape rejection, hash mismatch detection, and decompression bomb protection.
- **`rune-core` concurrency tests:** Verify Compare-and-Swap (CAS) race-condition prevention across simultaneous pushes.
- **`rune-core` property tests (`proptest`):** Verify 3-way merge correctness, merge base identification, and Myers diffing.
- **`rune-web` adversarial tests:** Verify Host/Origin header validation against DNS rebinding, URL-encoded path traversal rejection, and oversized blob protections.

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on code style, testing requirements, and the ADR process for proposing architectural changes.

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
