# ADR-005: Directed Acyclic Graph (DAG) 3-Way Merge with LCA and Myers Diffing

## Status
Accepted

## Date
2026-10-22

## Context
Merging diverging branches of development is a core capability of any version control system. When merging branch `A` into branch `B`, the engine must determine what changes occurred in each branch relative to a common ancestor, auto-resolve non-overlapping edits, and clearly flag conflicting changes with conflict markers.

## Decision
We implemented a 3-way merge strategy backed by Lowest Common Ancestor (LCA) search and Myers diffing:
1. **Lowest Common Ancestor Search (`find_merge_base`):**
   - Traverses the commit DAG backward from both branch tips using breadth-first search (BFS) with visited sets.
   - Identifies the most recent common ancestor commit (`merge base`). If no common ancestor exists, the merge is rejected to prevent merging disconnected histories.
2. **Myers Diff Algorithm (`similar` crate):**
   - Line-by-line diffing between `base` vs `head` and `base` vs `other` is computed using the Myers diff algorithm.
3. **Conflict Resolution:**
   - If a file or hunk changed on only one side, it is automatically resolved and merged.
   - If both branches modify the same line ranges with differing contents, standard conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>> <branch>`) are emitted into the working tree, and the merge command prompts the user to resolve before committing.
4. **Tree-Level Deletion / Addition Handling:**
   - Auto-resolves files added or deleted on only one branch. Files modified on one branch and deleted on another produce an explicit conflict.

## Alternatives Considered

### 2-Way Diff Merging (Blind Overwrite)
- **Pros:** Trivial to implement.
- **Cons:** Cannot distinguish between a deletion, an addition, or an intentional modification, leading to silent data overwrites.
- **Rejected because:** 3-way merging against a common ancestor is the foundation of reliable distributed version control.

### Recursive Merge Strategy (Merge-of-Merges)
- **Pros:** Handles criss-cross merge topologies by generating virtual merge bases.
- **Cons:** High complexity with edge-case ambiguities.
- **Rejected because:** Standard LCA 3-way merge resolves the overwhelming majority of everyday branching workflows while keeping merge logic auditable and deterministic.

## Consequences
- **Correctness:** Safe branch merges with deterministic ancestor resolution.
- **Auditable Conflicts:** Conflicts are clearly surfaced in the working tree without corrupting repository object storage.
- **User Experience:** Developers receive explicit feedback on auto-merged files, deleted files, and conflicted paths.
