//! `wt stack show` — render the branch stack as an ASCII tree.

use std::collections::HashMap;

use color_print::cformat;
use worktrunk::git::Repository;

use super::lineage::{collect_descendants, find_all_roots, find_stack_scope};

/// Render the stack tree containing the current branch.
pub fn show_stack(repo: &Repository, all: bool) -> anyhow::Result<()> {
    let current_branch = repo
        .current_worktree()
        .branch()
        .ok()
        .flatten();

    if all {
        show_all_stacks(repo, current_branch.as_deref())?;
    } else {
        let Some(branch) = &current_branch else {
            // Detached HEAD
            eprintln!("No stacked branches found (detached HEAD).");
            return Ok(());
        };

        let Some((root, stack_base)) = find_stack_scope(repo, branch) else {
            eprintln!("No stacked branches found. Use `wt switch -c <branch> --base <parent>` to create one.");
            return Ok(());
        };

        // Collect descendants only from the stack base, then add root → [stack_base]
        // so the tree shows root for context but excludes sibling stacks
        let mut children_map = collect_descendants(repo, &stack_base);
        children_map.insert(root.clone(), vec![stack_base]);
        render_tree(&root, &children_map, current_branch.as_deref());
    }

    Ok(())
}

fn show_all_stacks(repo: &Repository, current_branch: Option<&str>) -> anyhow::Result<()> {
    let roots = find_all_roots(repo);
    if roots.is_empty() {
        eprintln!("No stacked branches found. Use `wt switch -c <branch> --base <parent>` to create one.");
        return Ok(());
    }

    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let children_map = collect_descendants(repo, root);
        render_tree(root, &children_map, current_branch);
    }
    Ok(())
}

fn render_tree(
    root: &str,
    children_map: &HashMap<String, Vec<String>>,
    current_branch: Option<&str>,
) {
    println!("{}", format_node(root, current_branch));
    if let Some(children) = children_map.get(root) {
        render_children(children, children_map, current_branch, "");
    }
}

fn render_children(
    children: &[String],
    children_map: &HashMap<String, Vec<String>>,
    current_branch: Option<&str>,
    prefix: &str,
) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└─ " } else { "├─ " };
        let child_prefix = if is_last { "   " } else { "│  " };

        println!(
            "{}{}{}",
            prefix,
            connector,
            format_node(child, current_branch)
        );

        if let Some(grandchildren) = children_map.get(child.as_str()) {
            render_children(
                grandchildren,
                children_map,
                current_branch,
                &format!("{}{}", prefix, child_prefix),
            );
        }
    }
}

fn format_node(branch: &str, current_branch: Option<&str>) -> String {
    let is_current = current_branch == Some(branch);
    if is_current {
        cformat!("<bold>{branch}</>  <dim>← you are here</>")
    } else {
        branch.to_string()
    }
}
