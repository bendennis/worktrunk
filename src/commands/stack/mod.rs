//! Stack management commands.
//!
//! - `show` — Visualize the branch stack as an ASCII tree
//! - `set-parent` / `unset-parent` — Manage parent relationships
//! - `rebase` — Cascade rebase through the stack

pub(crate) mod lineage;
mod rebase;
mod show;

use anyhow::{Context, bail};
use color_print::cformat;
use worktrunk::git::Repository;
use worktrunk::styling::{eprintln, info_message, success_message};

use crate::cli::StackCommand;

pub fn handle_stack_command(cmd: StackCommand) -> anyhow::Result<()> {
    match cmd {
        StackCommand::Show { all } => {
            let repo = Repository::current()?;
            show::show_stack(&repo, all)
        }
        StackCommand::SetParent { parent, branch } => set_parent(parent, branch),
        StackCommand::UnsetParent { branch } => unset_parent(branch),
        StackCommand::Rebase { branch } => {
            let repo = Repository::current()?;
            rebase::cascade_rebase(&repo, branch.as_deref())
        }
    }
}

fn set_parent(parent: String, branch_arg: Option<String>) -> anyhow::Result<()> {
    let repo = Repository::current()?;

    let branch = match branch_arg {
        Some(b) => b,
        None => repo
            .current_worktree()
            .branch()
            .ok()
            .flatten()
            .context("Cannot determine current branch (detached HEAD?)")?,
    };

    // Validate: not self-referencing
    if branch == parent {
        bail!("Cannot set a branch as its own parent");
    }

    // Validate: parent branch exists
    if !repo.branch(&parent).exists()? {
        bail!(cformat!(
            "Branch <bold>{parent}</> does not exist"
        ));
    }

    // Validate: no cycles
    if lineage::would_create_cycle(&repo, &branch, &parent) {
        bail!(cformat!(
            "Setting <bold>{parent}</> as parent of <bold>{branch}</> would create a cycle"
        ));
    }

    repo.set_branch_parent(&branch, &parent)?;
    eprintln!(
        "{}",
        success_message(cformat!(
            "Set parent of <bold>{branch}</> to <bold>{parent}</>"
        ))
    );
    Ok(())
}

fn unset_parent(branch_arg: Option<String>) -> anyhow::Result<()> {
    let repo = Repository::current()?;

    let branch = match branch_arg {
        Some(b) => b,
        None => repo
            .current_worktree()
            .branch()
            .ok()
            .flatten()
            .context("Cannot determine current branch (detached HEAD?)")?,
    };

    let had_parent = repo.clear_branch_parent(&branch)?;
    if had_parent {
        eprintln!(
            "{}",
            success_message(cformat!(
                "Removed parent of <bold>{branch}</>"
            ))
        );
    } else {
        eprintln!(
            "{}",
            info_message(cformat!(
                "Branch <bold>{branch}</> has no parent set"
            ))
        );
    }
    Ok(())
}
