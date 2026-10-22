# Runestone User Manual (Rune Web UI)

## Overview

Runestone is a local, read-only web interface for browsing Rune repository history. It provides a visual way to explore commit timelines, file trees at any point in history, diffs, and branch overviews. 

It is rendered entirely in your browser and served from the data already present in your local `.rune/objects` store. Runestone operates **offline** and requires zero network access (no calls to GCS or remote servers are made).

## Key Features

- **Commit Timeline:** View a chronological list of commits for any branch, complete with author, date, and message.
- **File Tree Browser:** Click on any commit to browse the repository's directory structure exactly as it existed at that point in time.
- **File Viewer:** Open files directly in the browser with built-in syntax highlighting.
- **Diff Viewer:** Inspect changes between a commit and its parent, with additions and deletions clearly marked.
- **Branch Overview:** See all local and remote-tracking branches and easily switch contexts.
- **Security First:** Bound only to `127.0.0.1` (localhost) to ensure no external network access is possible.

> [!NOTE]
> Runestone is strictly **read-only**. You cannot commit, merge, push, or modify your repository from the web UI. All repository modifications must still be done via the `rune-cli` command line.

## Usage

### 1. Launching the Web Interface

Runestone is built into the Rune CLI toolset. To launch the web interface, navigate to your local Rune repository and run:

```bash
# From within a Rune repository
rune-cli web
```

This will start a local HTTP server (typically bound to `127.0.0.1:8080` or a similar local port). Depending on your configuration, this may automatically open a new tab in your default web browser.

### 2. Accessing the UI Manually

If your browser does not open automatically, look at the terminal output for the server address (e.g., `http://127.0.0.1:8080`) and open it manually in your web browser.

### 3. Browsing History

Once the UI is open, you can:
- **View Commits:** Scroll through the timeline on the main page.
- **Inspect Files:** Click on a commit hash to view the tree and click on individual files to see their contents.
- **View Diffs:** Navigate to a commit's detail page to see a side-by-side or unified diff of what changed in that commit.
- **Switch Branches:** Use the branch dropdown/overview page to switch between different lines of development.

## Security Considerations

- Runestone is designed to be resilient against malformed or malicious repository content (such as directory traversal attacks or "decompression bombs"). 
- It is safe to use against repositories you have cloned or fetched from untrusted sources, as the server incorporates strict path canonicalization and sanitization.

> [!TIP]
> Since Runestone reads directly from your `.rune` directory, you can leave the server running in the background while you continue to `rune-cli commit` or `rune-cli fetch` in your terminal. Just refresh your browser to see the latest changes!
