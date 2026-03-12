# Stacked Branch Support (`wt stack`)

## Summary

Adds `wt stack` — a set of commands for managing dependent branch chains (e.g., `main → feature-a → feature-b`). Parent-child relationships are stored in git config and used to automate rebase cascading, integrated branch detection, and push.

## Commands

| Command | Purpose |
|---------|---------|
| `wt stack set-parent <branch>` | Declare current branch's parent |
| `wt stack unset-parent` | Remove parent relationship |
| `wt stack show [--all]` | Visualize the stack as an ASCII tree |
| `wt stack sync` | Fetch, prune integrated branches, cascade rebase, push |

`wt stack sync` accepts `--no-fetch` and `--no-push` for partial runs.

## How it works

**Parent tracking** — Stored in git config as `worktrunk.state.<branch>.parent`. No new files or state formats.

**Automatic reparenting** — When a branch is merged or removed, its children are reparented to the grandparent. During sync, branches integrated into their parent (e.g., merged on GitHub) are detected and pruned from the stack automatically, with a hint to `wt remove` the stale worktree.

**Sync workflow:**
1. `git fetch --prune`
2. Detect branches integrated into their parent; reparent children
3. Cascade rebase through the current branch's stack (scoped to the stack base and its descendants, not all branches off the default branch)
4. Push changed branches with `--force-with-lease` (skips branches already in sync)

Sync errors if the current branch has no parent (e.g., running from `main`).

**Conflict handling** — Sync stops on conflict with a message pointing to the worktree to resolve in. After resolving and running `git rebase --continue`, re-run `wt stack sync`.

## Integration with existing commands

- `wt switch --create` automatically sets the parent (to the `--base` value or the default branch)
- `wt merge` and `wt remove` reparent children before cleanup
- `wt list --format=json` includes a `parent` field

## Files changed

**New files:**
- `src/cli/stack.rs` — CLI definitions
- `src/commands/stack/` — `mod.rs`, `lineage.rs`, `rebase.rs`, `show.rs`, `sync.rs`
- `tests/integration_tests/stack.rs` — 25 integration tests

**Modified files:**
- `src/git/repository/config.rs` — `branch_parent`, `set_branch_parent`, `clear_branch_parent`, `branch_children`, `reparent_children`
- `src/commands/merge.rs` — Reparent children on merge
- `src/commands/repository_ext.rs` — Reparent children on remove
- `src/commands/worktree/switch.rs` — Auto-set parent on branch creation
- `src/commands/list/` — `parent` field in list output and JSON
- `src/cli/mod.rs`, `src/main.rs` — Wire up `Stack` subcommand

No breaking changes to existing commands or interfaces.
