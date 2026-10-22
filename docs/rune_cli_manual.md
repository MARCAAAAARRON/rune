# RUNE User Manual

## Overview

Rune is a distributed Version Control System (VCS) written in Rust, inspired by Git but architected from the ground up to be safe, concurrent, and highly performant. Its key distinguishing feature is that it uses Google Cloud Storage (GCS) as its remote backend instead of requiring a dedicated server process. It targets developers who want Git-equivalent workflows (branch, commit, merge, push, pull) without operating remote infrastructure.

## Key Features

- **Serverless Remotes**: Uses Google Cloud Storage (GCS) directly for push/pull.
- **Cryptographically Secure:** All commits, trees, and blobs are content-addressable via SHA-256.
- **Robust Storage:** Objects are transparently compressed with Zstandard (zstd) and stored locally.
- **Path-Safe:** Immune to directory traversal attacks (`.` and `..` or slashes are strictly forbidden in tree entries).
- **Atomic Operations:** Ref updates use Compare-and-Swap (CAS) to prevent race conditions during concurrent pushes.

> [!NOTE]
> Rune is currently a standalone VCS and is **not** interoperable with standard Git clients or tooling.

## Installation

To build and install Rune from source, make sure you have the Rust toolchain installed, then run:

```bash
cargo install --path rune-cli
# The binary will be available in your PATH as `rune-cli`
```

## Basic Workflows

### 1. Initialize a Repository

To start using Rune in a project, navigate to your project directory and initialize it:

```bash
# Creates a new .rune directory in your project
rune-cli init
```

### 2. Stage and Commit Changes

To record a snapshot of your files:

```bash
# Add files to the index
rune-cli add src/main.rs
rune-cli add Cargo.toml

# Record the snapshot
rune-cli commit -m "Initial commit" --author "Your Name <your.email@example.com>"
```

### 3. Branching and Checkout

To work on features in isolation:

```bash
# List all branches
rune-cli branch

# Create a new branch
rune-cli branch feature-auth

# Switch to a branch
rune-cli checkout feature-auth
```

### 4. Viewing History and Diffs

To see what has changed over time:

```bash
# View the commit history
rune-cli log

# See changes between two commits (using their hashes)
rune-cli diff <old-hash> <new-hash>
```

### 5. Remote Collaboration

Rune supports two types of remote repositories: Google Cloud Storage (GCS) and local filesystem paths.

#### Option A: Google Cloud Storage (Enterprise)

> [!IMPORTANT]
> To interact with GCS, you must have a billing account configured on your Google Cloud project and your GCS service account credentials properly configured (usually via the `GOOGLE_APPLICATION_CREDENTIALS` environment variable).

Add a GCS remote and push your branch:
```bash
# Add a remote named 'origin' pointing to your GCS bucket
rune-cli remote add origin gcs://your-bucket-name

# Push your branch to the remote repository
rune-cli push origin main

# Fetch updates from a remote repository without merging
rune-cli fetch origin main

# Pull (Fetch + Merge) updates into your local branch
rune-cli pull origin main
```

#### Option B: Google Drive (Personal 5TB+ via Desktop App)

If you have a Google One or Google Workspace subscription with ample Google Drive space, you can bypass GCS and use your Drive for backups. Rune supports pushing to any local folder, so you can use the **Google Drive for Desktop** app to mount your Drive locally and let it sync in the background.

```bash
# Assuming Google Drive is mounted at G:\
# 1. First, create a folder in your Drive (e.g., G:\My Drive\rune-backups)

# 2. Add the local Drive folder as a remote
rune-cli remote add origin "G:\My Drive\rune-backups"

# 3. Push your branch
rune-cli push origin main
```

## Security and Integrity

Rune takes data integrity very seriously:
- **Fetch Verification**: During a `fetch`, Rune streams the objects, decompresses them in memory, and cryptographically verifies that the SHA-256 hash strictly matches the remote's claim *before* parsing the object.
- **Push Concurrency**: Pushing to a remote transmits the current branch tip. If another user pushes simultaneously, Rune relies on **Generation Preconditions / Compare-and-Swap** to reject the conflicting push. This guarantees no silent data loss.

> [!WARNING]
> Do not store plaintext credentials in your repository. Ensure your GCS credentials are kept safe and you are using least-privilege IAM roles for your bucket.
