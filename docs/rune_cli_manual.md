# RUNE User Manual

## Overview

Rune is a distributed Version Control System (VCS) written in Rust, inspired by Git's content-addressable data model but architected from the ground up for safety, strict path sandboxing, and direct serverless cloud object storage.

Its distinguishing feature is direct synchronization with Google Cloud Storage (GCS) or shared directories (such as Google Drive for Desktop) without requiring a dedicated server process or daemon. It provides developers with standard VCS workflows (branch, commit, diff, merge, push, pull) without operating remote infrastructure.

> [!NOTE]
> Rune is a standalone VCS with its own content-addressable storage format. It is **not** interoperable with standard Git clients, remotes, or tooling.

---

## Key Features

- **Serverless Remotes**: Direct push/pull against Google Cloud Storage (`gcs://<bucket>`) or local paths without running remote servers.
- **Cryptographically Secure**: Commits, trees, and blobs are content-addressed via SHA-256. Object integrity is validated upon decompression prior to parsing.
- **Transparent Compression**: Objects are compressed locally and on remotes using Zstandard (`zstd`).
- **Path-Safe Merkle Trees**: Tree entries strictly forbid path separators (`/`, `\`) and traversal symbols (`.` and `..`), preventing malicious directory escape attacks.
- **Race-Condition-Resistant (Atomic CAS)**: Branch updates use Compare-and-Swap (CAS) with GCS generation preconditions or atomic filesystem operations, eliminating concurrent push conflicts.
- **Integrated Local Web UI**: Includes Runestone (`rune-cli web`), a zero-network, read-only browser UI to inspect histories, trees, and diffs.

---

## Installation & Setup

### Prerequisites

- [Rust Toolchain](https://www.rust-lang.org/) (edition 2021, Rust 1.70+)
- (Optional) [Google Cloud CLI](https://cloud.google.com/sdk) for GCS bucket authentication (`gcloud auth application-default login`)

### Build and Install

```bash
# Clone the repository
git clone https://github.com/MARCAAAAARRON/rune.git
cd rune

# Build release binaries
cargo build --release

# Install rune-cli to your Cargo binary directory
cargo install --path rune-cli
```

Verify that the CLI is installed and available in your `PATH`:
```bash
rune-cli --help
```

---

## Command Reference

| Command | Syntax | Description |
|---|---|---|
| `init` | `rune-cli init` | Initializes a new `.rune` repository in the current directory. |
| `add` | `rune-cli add <path>` | Stages a file or directory into the index (honoring `.runeignore`). |
| `commit` | `rune-cli commit -m "<msg>" [--author "<name <email>>"]` | Commits staged changes as a snapshot. |
| `log` | `rune-cli log` | Displays chronological commit history for the current branch. |
| `branch` | `rune-cli branch [name] [-d/--delete]` | Lists branches, creates a new branch, or deletes an existing branch. |
| `checkout` | `rune-cli checkout <target>` | Switches working tree and HEAD to a branch name or commit hash. |
| `diff` | `rune-cli diff <old-commit> <new-commit>` | Displays file changes and line-level unified diffs between two commits. |
| `merge` | `rune-cli merge <branch>` | Merges the specified branch into the current HEAD using 3-way LCA. |
| `remote` | `rune-cli remote add <name> <url>` | Registers a remote repository alias (e.g., `origin` to a GCS bucket or folder). |
| `remote` | `rune-cli remote list` | Lists all configured remote aliases and target endpoints. |
| `push` | `rune-cli push <remote> <branch>` | Pushes missing objects and updates branch refs on the remote. |
| `fetch` | `rune-cli fetch <remote> <branch>` | Downloads remote objects and updates remote tracking refs without merging. |
| `pull` | `rune-cli pull <remote> <branch>` | Fetches objects from the remote and merges them into the current branch. |
| `web` | `rune-cli web [--port 8080]` | Launches the Runestone local read-only web UI on `127.0.0.1`. |

---

## Everyday Workflows

### 1. Initialize a Repository

Navigate to your project root and run:
```bash
rune-cli init
```
This creates the hidden `.rune/` directory containing `objects/`, `refs/`, `HEAD`, `config`, and `index`.

### 2. Stage and Commit Changes

Stage files or entire directories into the staging area:
```bash
# Stage specific files or directories
rune-cli add src/main.rs
rune-cli add Cargo.toml

# Commit with a descriptive message and author signature
rune-cli commit -m "feat: implement user authentication" --author "Alice <alice@example.com>"
```
*(If `--author` is omitted, the default author `Rune User <user@example.com>` is used).*

### 3. Branching and Checkout

```bash
# List all existing branches
rune-cli branch

# Create a new feature branch
rune-cli branch feature-metrics

# Switch to the new branch
rune-cli checkout feature-metrics

# Delete a branch once merged
rune-cli branch feature-metrics --delete
```

### 4. Inspecting History and Diffs

```bash
# View the commit timeline
rune-cli log

# Compare changes between two commits
rune-cli diff <old-commit-hash> <new-commit-hash>
```

### 5. Merging Branches

Rune utilizes a 3-way merge strategy based on the Lowest Common Ancestor (LCA) in the commit DAG:
```bash
# Ensure you are on the target branch
rune-cli checkout main

# Merge the feature branch into main
rune-cli merge feature-metrics
```

- If changes do not conflict, files are automatically updated and staged.
- If both branches modify identical line ranges, conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>>`) are written directly to the conflicted files.
- Resolve any markers, stage the resolved files (`rune-cli add <path>`), and run `rune-cli commit -m "merge: resolve conflicts"`.

---

## Remote Collaboration

Rune supports two primary remote types:

### Option A: Google Cloud Storage (`gcs://`)
Ideal for serverless team synchronization without maintaining dedicated Git servers:

```bash
# 1. Authenticate with Google Cloud
gcloud auth application-default login

# 2. Add your GCS bucket as remote 'origin'
rune-cli remote add origin gcs://my-team-rune-bucket

# 3. Push your branch
rune-cli push origin main

# 4. Fetch or Pull
rune-cli fetch origin main
rune-cli pull origin main
```

> [!TIP]
> If you run `rune-cli push origin main` before configuring an `origin` remote, Rune will interactively prompt you to enter your GCS bucket name and configure it automatically!

### Option B: Local Filesystem or Cloud Sync Folders
Rune can push and pull directly from any directory, external drive, or cloud mount (such as Google Drive for Desktop):

```bash
# Add a mounted cloud folder as remote
rune-cli remote add origin "G:\My Drive\rune-backups"

# Push and pull
rune-cli push origin main
rune-cli pull origin main
```

---

## Runestone Web Interface

To visually browse commits, tree hierarchies, syntax-highlighted code, and diffs in your web browser:
```bash
rune-cli web --port 8080
```
Open `http://127.0.0.1:8080` in your web browser. Runestone binds exclusively to `127.0.0.1` and enforces strict Host/Origin header validation to protect against DNS rebinding attacks.

For more details on the web UI, see the [Runestone Web Manual](runestone_web_manual.md).

---

## Security & Architecture

For deeper technical details and design rationale:
- [Architecture Decision Records (ADRs)](decisions/README.md)
- [Technical Requirements Document (TRD)](Rune_TRD.md)
- [Product Requirements Document (PRD)](Rune_PRD.md)
