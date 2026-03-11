# Stacked Branches Implementation Plan

Native stacked branch support for worktrunk — track parent-child branch relationships
and cascade operations through the stack.

## Overview

New `wt stack` subcommand family plus parent-awareness wired into existing commands.
Each stacked branch has its own worktree, enabling in-place rebasing without checkout
gymnastics.

```
wt stack
├── sync         # fetch + rebase entire stack + push each branch
├── show         # visualize the stack as a tree
├── rebase       # cascade rebase through the stack (no fetch/push)
├── set-parent   # manually set parent for current branch
└── unset-parent # detach from parent
```

## Dependency Graph

```
Phase 1 (foundation)
├── Phase 2 (reparenting)     ← parallel with Phase 3
├── Phase 3 (wt stack + viz)
│   └── Phase 4 (cascade rebase)
│       └── Phase 5 (sync)
```

---

## Phase 1: Parent Storage + Automatic Targeting

**Branch:** `stack-phase-1-parent-storage`

**What ships:** `wt switch -c feature --base parent` persists the parent relationship.
All target-dependent commands (`wt step rebase`, `wt step squash`, `wt step diff`,
`wt step push`, `wt merge`) automatically resolve to the parent instead of the default
branch. `wt config state parent get/set/clear` for manual management.

**Why this is the foundation:** `resolve_target_branch()` is the single funnel for every
command that needs a target. One change there makes the entire existing command surface
stack-aware.

### Files to modify

| File | Change |
|------|--------|
| `src/git/repository/config.rs` | Add `branch_parent()`, `set_branch_parent()`, `clear_branch_parent()`. Modify `resolve_target_branch()`: explicit target → stored parent → default branch. |
| `src/commands/worktree/switch.rs` | After `CreationMethod::Regular { create_branch: true, base_branch: Some(..) }` succeeds (~line 780), call `repo.set_branch_parent(&branch, base)`. |
| `src/cli/config.rs` | Add `Parent` variant to `StateCommand` with `ParentAction` enum (Get/Set/Clear), following `MarkerAction` pattern. |
| `src/commands/config/state.rs` | Handle `"parent"` key in state get/set/clear/show dispatchers. |
| `src/main.rs` | Wire `StateCommand::Parent` dispatch. |

### Key decisions

- Parent stored as plain string in `worktrunk.state.<branch>.parent` (not JSON — no metadata needed).
- If stored parent branch no longer exists, `resolve_target_branch` silently falls back to default branch with a debug log.
- `--base` without `--create` still has no effect (current behavior preserved).

### Tests

- Unit tests for `branch_parent` / `set_branch_parent` / `clear_branch_parent`.
- Integration: create with `--base`, verify via `config state parent get`.
- Integration: `main → A → B`, verify `wt step rebase` from B targets A.
- Integration: parent branch gone, falls back to default.
- Snapshots for `wt config state parent` subcommands.

---

## Phase 2: Reparenting on Merge and Remove

**Branch:** `stack-phase-2-reparenting` (stacked on Phase 1)

**What ships:** When `wt merge` lands a branch, its children are automatically reparented
to the grandparent. Same for `wt remove`. No orphaned branches in the stack.

### Files to modify

| File | Change |
|------|--------|
| `src/git/repository/config.rs` | Add `branch_children(parent)` (via `git config --get-regexp`) and `reparent_children(old_parent, new_parent)`. |
| `src/commands/merge.rs` | After `handle_push` (~line 231), reparent children to `target_branch`, clear merged branch's parent. Info message if children reparented. |
| `src/commands/worktree/remove.rs` | Reparent children to the removed branch's own parent (grandparent), or clear if no grandparent. |
| `src/commands/step_commands.rs` | In `step_prune`, reparent children of pruned branches. |

### Key decisions

- On merge: children reparent to `target_branch` (where the parent's content now lives).
- On remove: children reparent to the removed branch's parent (grandparent), or detach if no grandparent.
- Info message shown, no confirmation needed.

### Tests

- `main → A → B → C`, merge B into A, verify C's parent is now A.
- `main → A → B`, remove A, verify B reparented.
- Merge a leaf branch with no children — no reparenting output.
- `wt step prune` reparents.

---

## Phase 3: `wt stack` Subcommand + List Visualization

**Branch:** `stack-phase-3-stack-command` (stacked on Phase 1)

**What ships:** `wt stack show` renders the stack as an ASCII tree. `wt stack set-parent`
and `wt stack unset-parent` as ergonomic alternatives. `wt list` shows parent in output
and JSON.

### Files to create

| File | Purpose |
|------|---------|
| `src/cli/stack.rs` | CLI definitions: `StackCommand` enum with `Show`, `SetParent`, `UnsetParent`. |
| `src/commands/stack/mod.rs` | Module root, dispatches to subcommands. |
| `src/commands/stack/show.rs` | Tree rendering — walks up to root, renders full tree down. |
| `src/commands/stack/lineage.rs` | Shared lineage utilities: `collect_ancestors()`, `collect_descendants()`, `detect_cycle()`. Phase 4 reuses this. |

### Files to modify

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Add `mod stack;`, `Stack` variant to `Commands` enum. |
| `src/commands/mod.rs` | Add `pub(crate) mod stack;`. |
| `src/main.rs` | Dispatch `Commands::Stack`. |
| `src/commands/list/model/item.rs` | Add `parent: Option<String>` to `ListItem`. |
| `src/commands/list/collect/` | Populate `item.parent` from `repo.branch_parent(branch)`. |
| `src/commands/list/json_output.rs` | Include `parent` in JSON output (skip when None). |
| `src/commands/list/render.rs` | Show `← parent` after branch name when parent is set. |

### `wt stack show` output

```
main
└─ feature-a
   ├─ feature-b  ← you are here
   └─ feature-c
```

### Key decisions

- `set-parent` validates: parent exists, not self-referencing, no cycles.
- `show` without arguments walks up from current branch to root, renders full tree.
- `lineage.rs` extracted because Phase 4 needs the same tree-walking.
- In `wt list`, parent info is subtle (`← parent` inline). Full tree view lives in `wt stack show`.

### Tests

- Snapshot tests for `wt stack show` with various topologies.
- Snapshot tests for `wt list` with parents present.
- Cycle detection in `set-parent`.
- JSON output test verifying `parent` field.

---

## Phase 4: `wt stack rebase` — Cascade Rebase

**Branch:** `stack-phase-4-cascade-rebase` (stacked on Phase 3)

**What ships:** `wt stack rebase` rebases the current branch onto its parent, then
cascades down to all descendants in topological order. Each rebase happens in-place
in the branch's worktree.

### Files to create/modify

| File | Purpose |
|------|---------|
| `src/cli/stack.rs` | Add `Rebase` variant with `branch: Option<String>`, `--yes`. |
| `src/commands/stack/rebase.rs` | Core cascade logic using `collect_descendants()` from lineage.rs. |
| `src/commands/stack/mod.rs` | Wire dispatch. |

### Algorithm

1. Start from current branch (or `--branch`).
2. If it has a parent, rebase onto parent.
3. BFS collect all descendants.
4. For each descendant (BFS order ensures parent before child):
   - Look up worktree path.
   - If no worktree → error: "run `wt switch <branch>` first".
   - Run `git rebase <parent>` in that worktree via `Cmd::new("git").current_dir(worktree_path)`.
   - On conflict → stop, tell user which worktree to resolve in.

### Key decisions

- Only operates on branches with worktrees (safety + conflict resolution needs a working directory).
- On conflict, stops at the failing branch. Re-run picks up where it left off.
- BFS ordering guarantees parents processed before children.

### Tests

- Linear stack: `main → A → B → C`, advance main, verify all rebase.
- Branching: `A → B, A → C`, verify both children rebase.
- Conflict stops cascade, already-rebased branches are no-ops on re-run.
- Child missing worktree → error.

---

## Phase 5: `wt stack sync` — Full Sync

**Branch:** `stack-phase-5-sync` (stacked on Phase 4)

**What ships:** `wt stack sync` — the everyday "update my stack" command. Fetches remote,
rebases entire stack from root down, pushes each branch with `--force-with-lease`.

### Files to create/modify

| File | Purpose |
|------|---------|
| `src/cli/stack.rs` | Add `Sync` variant with `--yes`, `--no-push`, `--no-fetch`. |
| `src/commands/stack/sync.rs` | Orchestrator: fetch → cascade rebase (Phase 4) → push loop. |

### Algorithm

1. Walk up to find stack root.
2. `git fetch` (unless `--no-fetch`).
3. Cascade rebase from root (calls Phase 4 logic).
4. For each branch in stack (unless `--no-push`):
   - `git push --force-with-lease` in the branch's worktree.

### Key decisions

- `--force-with-lease` mandatory for post-rebase pushes (safe variant of force push).
- `--no-push` for local-only updates.
- `--no-fetch` for offline operation.
- Thin orchestration layer on Phase 4.

### Tests

- Full sync cycle with remote fixture.
- Snapshot test for output progression.
