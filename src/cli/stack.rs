use clap::Subcommand;

/// Manage stacked branches
#[derive(Subcommand)]
pub enum StackCommand {
    /// Show the branch stack as a tree
    #[command(
        after_long_help = r#"Visualizes the stacked branch hierarchy as an ASCII tree rooted at the default branch.

## Examples

```console
wt stack show              # Show stack containing current branch
wt stack show --all        # Show all stacks
```

## Output

```
main
└─ feature-a
   ├─ feature-b  ← you are here
   └─ feature-c
```

Branches without a stored parent are not shown unless `--all` is used.
"#
    )]
    Show {
        /// Show all stacks, not just the current branch's
        #[arg(long)]
        all: bool,
    },

    /// Set the parent of the current branch
    #[command(
        after_long_help = r#"Sets the parent branch for stacking. The parent is used as the default target for rebase, merge, and diff operations.

## Examples

```console
wt stack set-parent main          # Set parent to main
wt stack set-parent feature-a     # Stack on feature-a
```

Validates that the parent exists, is not the current branch, and would not create a cycle.
"#
    )]
    SetParent {
        /// Parent branch name
        #[arg(add = crate::completion::branch_value_completer())]
        parent: String,

        /// Branch to set parent for (defaults to current)
        #[arg(long, add = crate::completion::branch_value_completer())]
        branch: Option<String>,
    },

    /// Remove the parent of the current branch
    #[command(
        after_long_help = r#"Detaches the branch from its parent. Target resolution falls back to the default branch.

## Examples

```console
wt stack unset-parent             # Detach current branch
wt stack unset-parent --branch feature-b
```
"#
    )]
    UnsetParent {
        /// Branch to unset parent for (defaults to current)
        #[arg(long, add = crate::completion::branch_value_completer())]
        branch: Option<String>,
    },

    /// Rebase the stack from current branch down
    ///
    /// Rebases onto parent, then cascades to all descendants in topological order.
    #[command(
        after_long_help = r#"Rebases the current branch onto its parent, then cascades down through all descendants. Each branch is rebased in its own worktree.

## Examples

```console
wt stack rebase                   # Rebase from current branch down
wt stack rebase --branch feature  # Start from a specific branch
```

## Behavior

1. Rebase current branch onto its parent (skipped if no parent)
2. For each descendant (BFS order): rebase onto its parent
3. On conflict: stop and report which worktree needs resolution

After resolving conflicts, re-run `wt stack rebase` to continue — already-rebased branches are skipped.

Every branch in the stack must have a worktree. Use `wt switch <branch>` to create missing worktrees before rebasing.
"#
    )]
    Rebase {
        /// Start from this branch instead of current
        #[arg(long, add = crate::completion::branch_value_completer())]
        branch: Option<String>,
    },

    /// Fetch, rebase entire stack, and push each branch
    ///
    /// The everyday "update my stack" command: fetch, cascade rebase from root, push with --force-with-lease.
    #[command(
        after_long_help = r#"Fetches from the remote, rebases the entire stack from root down, then pushes each branch with `--force-with-lease`.

## Examples

```console
wt stack sync                    # Full sync
wt stack sync --no-push          # Rebase only, don't push
wt stack sync --no-fetch         # Skip fetch (offline)
```

## Behavior

1. `git fetch --prune` (unless `--no-fetch`)
2. Cascade rebase from stack root (same as `wt stack rebase`)
3. Push each branch with `--force-with-lease` (unless `--no-push`)
   - Branches without an upstream get `-u origin <branch>` instead

On conflict during rebase, sync stops. Resolve and re-run.
"#
    )]
    Sync {
        /// Skip fetch
        #[arg(long)]
        no_fetch: bool,

        /// Skip push (rebase only)
        #[arg(long)]
        no_push: bool,
    },
}
