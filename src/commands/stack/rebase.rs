//! `wt stack rebase` — cascade rebase through the stack.
//!
//! Rebases the current branch onto its parent, then cascades down to all
//! descendants in BFS order. Each rebase runs in the branch's worktree.

use std::collections::VecDeque;

use anyhow::{Context, bail};
use color_print::cformat;
use worktrunk::git::Repository;
use worktrunk::styling::{eprintln, info_message, progress_message, success_message};

use super::lineage::collect_descendants;

/// Run cascade rebase starting from `start_branch` (or current branch).
pub fn cascade_rebase(repo: &Repository, start_branch: Option<&str>) -> anyhow::Result<()> {
    let branch = match start_branch {
        Some(b) => b.to_string(),
        None => repo
            .current_worktree()
            .branch()
            .ok()
            .flatten()
            .context("Cannot determine current branch (detached HEAD?)")?,
    };

    // Step 1: Rebase start branch onto its parent (if it has one)
    if let Some(parent) = repo.branch_parent(&branch) {
        rebase_branch(repo, &branch, &parent)?;
    }

    // Step 2: BFS cascade through descendants
    let children_map = collect_descendants(repo, &branch);
    if children_map.is_empty() {
        return Ok(());
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    if let Some(children) = children_map.get(&branch) {
        queue.extend(children.iter().cloned());
    }

    let mut rebased_count = 0usize;
    while let Some(child) = queue.pop_front() {
        let parent = repo
            .branch_parent(&child)
            .context(cformat!(
                "Branch <bold>{child}</> lost its parent during cascade"
            ))?;

        if rebase_branch(repo, &child, &parent)? {
            rebased_count += 1;
        }

        // Enqueue grandchildren
        if let Some(grandchildren) = children_map.get(&child) {
            queue.extend(grandchildren.iter().cloned());
        }
    }

    if rebased_count > 0 {
        eprintln!(
            "{}",
            success_message(cformat!(
                "Rebased {rebased_count} descendant branch{}",
                if rebased_count == 1 { "" } else { "es" }
            ))
        );
    }

    Ok(())
}

/// Rebase a single branch onto its parent in that branch's worktree.
/// Returns `true` if a rebase was performed, `false` if already up-to-date.
fn rebase_branch(repo: &Repository, branch: &str, parent: &str) -> anyhow::Result<bool> {
    let worktree_path = repo.worktree_for_branch(branch)?.ok_or_else(|| {
        anyhow::anyhow!(cformat!(
            "Branch <bold>{branch}</> has no worktree. Run <bold>wt switch {branch}</> first."
        ))
    })?;

    let wt = repo.worktree_at(&worktree_path);

    // Check if already rebased
    let merge_base = wt
        .run_command(&["merge-base", "HEAD", parent])
        .ok()
        .map(|s| s.trim().to_string());
    let parent_sha = wt
        .run_command(&["rev-parse", parent])
        .ok()
        .map(|s| s.trim().to_string());

    if merge_base.is_some() && merge_base == parent_sha {
        // Check for merge commits that indicate non-linear history
        let merge_commits = wt
            .run_command(&["rev-list", "--merges", &format!("{parent}..HEAD")])
            .unwrap_or_default();
        if merge_commits.trim().is_empty() {
            eprintln!(
                "{}",
                info_message(cformat!(
                    "<bold>{branch}</> already up-to-date with <bold>{parent}</>"
                ))
            );
            return Ok(false);
        }
    }

    eprintln!(
        "{}",
        progress_message(cformat!(
            "Rebasing <bold>{branch}</> onto <bold>{parent}</>..."
        ))
    );

    let result = wt.run_command(&["rebase", parent]);

    if let Err(e) = result {
        // Check if it's a conflict
        let is_rebasing = wt
            .run_command(&["status", "--porcelain=v2", "--branch"])
            .ok()
            .map(|s| s.contains("rebase"))
            .unwrap_or(false);

        if is_rebasing {
            bail!(cformat!(
                "Rebase conflict in <bold>{branch}</>. Resolve in:\n  {}\nThen re-run <bold>wt stack sync</> to continue.",
                worktree_path.display()
            ));
        }
        return Err(e).context(cformat!(
            "Failed to rebase <bold>{branch}</> onto <bold>{parent}</>\nResolve conflicts, then re-run <bold>wt stack sync</>"
        ));
    }

    Ok(true)
}
