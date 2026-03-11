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
}
