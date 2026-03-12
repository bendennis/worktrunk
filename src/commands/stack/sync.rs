//! `wt stack sync` — fetch, cascade rebase, push entire stack.

use std::collections::{HashSet, VecDeque};

use anyhow::Context;
use color_print::cformat;
use worktrunk::git::Repository;
use worktrunk::styling::{
    eprintln, hint_message, info_message, progress_message, success_message, warning_message,
};

use super::lineage::{collect_descendants, find_stack_root};
use super::rebase::cascade_rebase;

pub fn stack_sync(
    repo: &Repository,
    no_fetch: bool,
    no_push: bool,
) -> anyhow::Result<()> {
    let current_branch = repo
        .current_worktree()
        .branch()
        .ok()
        .flatten()
        .context("Cannot determine current branch (detached HEAD?)")?;

    let root = find_stack_root(repo, &current_branch);

    // Step 1: Fetch
    if !no_fetch {
        eprintln!(
            "{}",
            progress_message("Fetching from remote...")
        );
        match repo.run_command(&["fetch", "--prune"]) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "{}",
                    warning_message(format!("Fetch failed: {e}"))
                );
            }
        }
    }

    // Step 2: Prune integrated branches from the stack
    let pruned = prune_integrated_branches(repo, &root)?;

    // Step 3: Cascade rebase from root
    cascade_rebase(repo, Some(&root))?;

    // Step 4: Push each branch with --force-with-lease
    if !no_push {
        let branches = collect_stack_branches(repo, &root);
        for branch in &branches {
            if pruned.contains(branch) {
                continue;
            }
            let wt_path = match repo.worktree_for_branch(branch)? {
                Some(p) => p,
                None => continue,
            };
            let wt = repo.worktree_at(&wt_path);

            // Check if branch has a remote tracking branch
            let has_upstream = wt
                .run_command(&["rev-parse", "--abbrev-ref", &format!("{branch}@{{u}}")])
                .is_ok();

            if !has_upstream {
                // No upstream — push with -u to set it
                eprintln!(
                    "{}",
                    progress_message(cformat!(
                        "Pushing <bold>{branch}</> (setting upstream)..."
                    ))
                );
                if let Err(e) = wt.run_command(&["push", "-u", "origin", branch]) {
                    eprintln!(
                        "{}",
                        warning_message(cformat!(
                            "Failed to push <bold>{branch}</>: {e}"
                        ))
                    );
                }
            } else {
                // Check if local differs from upstream before pushing
                let local_sha = wt
                    .run_command(&["rev-parse", "HEAD"])
                    .ok()
                    .map(|s| s.trim().to_string());
                let upstream_sha = wt
                    .run_command(&["rev-parse", &format!("{branch}@{{u}}")])
                    .ok()
                    .map(|s| s.trim().to_string());

                if local_sha.is_some() && local_sha == upstream_sha {
                    continue;
                }

                // Has upstream but differs — force-with-lease (safe for post-rebase)
                eprintln!(
                    "{}",
                    progress_message(cformat!(
                        "Pushing <bold>{branch}</>..."
                    ))
                );
                if let Err(e) = wt.run_command(&["push", "--force-with-lease"]) {
                    eprintln!(
                        "{}",
                        warning_message(cformat!(
                            "Failed to push <bold>{branch}</>: {e}"
                        ))
                    );
                }
            }
        }
    }

    eprintln!("{}", success_message("Sync complete"));

    Ok(())
}

/// Detect and prune branches that have been integrated into their parent
/// (e.g., merged on GitHub). Reparents children to the integrated branch's
/// parent, keeping the stack intact. Returns the set of pruned branch names.
fn prune_integrated_branches(
    repo: &Repository,
    root: &str,
) -> anyhow::Result<HashSet<String>> {
    let children_map = collect_descendants(repo, root);
    let mut pruned = HashSet::new();

    // BFS from root, checking each non-root branch
    let mut queue: VecDeque<String> = VecDeque::new();
    if let Some(children) = children_map.get(root) {
        queue.extend(children.iter().cloned());
    }

    while let Some(branch) = queue.pop_front() {
        // Look up the current parent (may have been reparented by a previous iteration)
        let Some(parent) = repo.branch_parent(&branch) else {
            // Enqueue children even if this branch has no parent
            if let Some(children) = children_map.get(&branch) {
                queue.extend(children.iter().cloned());
            }
            continue;
        };

        // Check if branch is integrated into its parent
        let is_integrated = repo
            .integration_reason(&branch, &parent)
            .ok()
            .and_then(|(_, reason)| reason)
            .is_some();

        if is_integrated {
            let reparented = repo.reparent_children(&branch, Some(&parent))?;
            let _ = repo.clear_branch_parent(&branch);

            let reparent_msg = if reparented > 0 {
                cformat!(
                    "; reparented {reparented} child branch{} to <bold>{parent}</>",
                    if reparented == 1 { "" } else { "es" }
                )
            } else {
                String::new()
            };

            eprintln!(
                "{}",
                info_message(cformat!(
                    "<bold>{branch}</> integrated into <bold>{parent}</>{reparent_msg}"
                ))
            );
            eprintln!(
                "{}",
                hint_message(cformat!(
                    "To remove, run <underline>wt remove {branch}</>"
                ))
            );

            pruned.insert(branch.clone());
        }

        // Always enqueue children (they may have been reparented but still need visiting)
        if let Some(children) = children_map.get(&branch) {
            queue.extend(children.iter().cloned());
        }
    }

    Ok(pruned)
}

/// Collect all branches in the stack starting from root, BFS order.
fn collect_stack_branches(repo: &Repository, root: &str) -> Vec<String> {
    let children_map = collect_descendants(repo, root);
    let mut branches = vec![root.to_string()];
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(parent) = queue.pop_front() {
        if let Some(children) = children_map.get(&parent) {
            for child in children {
                branches.push(child.clone());
                queue.push_back(child.clone());
            }
        }
    }

    branches
}
