//! `wt stack sync` — fetch, cascade rebase, push entire stack.

use std::collections::VecDeque;

use anyhow::Context;
use color_print::cformat;
use worktrunk::git::Repository;
use worktrunk::styling::{eprintln, progress_message, success_message, warning_message};

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

    // Step 2: Cascade rebase from root
    cascade_rebase(repo, Some(&root))?;

    // Step 3: Push each branch with --force-with-lease
    if !no_push {
        let branches = collect_stack_branches(repo, &root);
        let mut pushed = 0usize;
        let mut skipped = 0usize;

        for branch in &branches {
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
                match wt.run_command(&["push", "-u", "origin", branch]) {
                    Ok(_) => pushed += 1,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            warning_message(cformat!(
                                "Failed to push <bold>{branch}</>: {e}"
                            ))
                        );
                    }
                }
            } else {
                // Has upstream — force-with-lease (safe for post-rebase)
                eprintln!(
                    "{}",
                    progress_message(cformat!(
                        "Pushing <bold>{branch}</>..."
                    ))
                );
                match wt.run_command(&["push", "--force-with-lease"]) {
                    Ok(_) => pushed += 1,
                    Err(e) => {
                        // Check if already up-to-date
                        let err_str = e.to_string();
                        if err_str.contains("Everything up-to-date") {
                            skipped += 1;
                        } else {
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
        }

        if pushed > 0 || skipped > 0 {
            let mut parts = Vec::new();
            if pushed > 0 {
                parts.push(format!(
                    "pushed {pushed} branch{}",
                    if pushed == 1 { "" } else { "es" }
                ));
            }
            if skipped > 0 {
                parts.push(format!(
                    "{skipped} already up-to-date",
                ));
            }
            eprintln!("{}", success_message(cformat!("Sync complete: {}", parts.join(", "))));
        }
    }

    Ok(())
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
