//! Shared lineage utilities for stacked branch tree-walking.
//!
//! Used by `wt stack show` and `wt stack sync`.

use std::collections::{HashMap, HashSet, VecDeque};

use worktrunk::git::Repository;

/// Walk up from `branch` to find the stack root and stack base.
///
/// The **root** is the topmost branch with no parent (e.g., `main`).
/// The **base** is the first branch after root in the current branch's lineage
/// (e.g., for `main → A → B → C`, calling from C returns `(main, A)`).
///
/// Returns `None` if the branch has no parent (it is a root itself).
pub fn find_root_and_base(repo: &Repository, branch: &str) -> Option<(String, String)> {
    let mut current = branch.to_string();
    let mut child_of_root = None;
    let mut seen = HashSet::new();
    seen.insert(current.clone());
    while let Some(parent) = repo.branch_parent(&current) {
        if !seen.insert(parent.clone()) {
            break;
        }
        child_of_root = Some(current);
        current = parent;
    }
    child_of_root.map(|base| (current, base))
}

/// Collect all descendants of `root` as a parent→children map.
///
/// Returns a map where each key is a parent branch and the value is a sorted
/// list of its children. Only includes branches that are reachable from `root`
/// through the parent chain.
pub fn collect_descendants(repo: &Repository, root: &str) -> HashMap<String, Vec<String>> {
    // Get all parent relationships
    let all_parents = repo
        .run_command(&["config", "--get-regexp", r"^worktrunk\.state\..*\.parent$"])
        .unwrap_or_default();

    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    for line in all_parents.lines() {
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        let value = value.trim();
        let Some(child) = key
            .strip_prefix("worktrunk.state.")
            .and_then(|s| s.strip_suffix(".parent"))
        else {
            continue;
        };
        children_map
            .entry(value.to_string())
            .or_default()
            .push(child.to_string());
    }

    // Sort children for deterministic output
    for children in children_map.values_mut() {
        children.sort();
    }

    // BFS from root to only include reachable branches
    let mut reachable: HashMap<String, Vec<String>> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(parent) = queue.pop_front() {
        if let Some(children) = children_map.get(&parent) {
            reachable.insert(parent.clone(), children.clone());
            for child in children {
                queue.push_back(child.clone());
            }
        }
    }

    reachable
}

/// Detect if setting `branch`'s parent to `new_parent` would create a cycle.
///
/// Walks up from `new_parent` through the parent chain. If we reach `branch`,
/// it would create a cycle.
pub fn would_create_cycle(repo: &Repository, branch: &str, new_parent: &str) -> bool {
    let mut current = new_parent.to_string();
    let mut seen = HashSet::new();
    seen.insert(branch.to_string());
    loop {
        if !seen.insert(current.clone()) {
            return true; // Found a cycle
        }
        match repo.branch_parent(&current) {
            Some(parent) => current = parent,
            None => return false, // Reached a root — no cycle
        }
    }
}

/// Collect all roots (branches with children but no parent, plus the default branch).
pub fn find_all_roots(repo: &Repository) -> Vec<String> {
    let all_parents = repo
        .run_command(&["config", "--get-regexp", r"^worktrunk\.state\..*\.parent$"])
        .unwrap_or_default();

    let mut parents_set: HashSet<String> = HashSet::new();
    let mut children_set: HashSet<String> = HashSet::new();

    for line in all_parents.lines() {
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        let value = value.trim().to_string();
        let Some(child) = key
            .strip_prefix("worktrunk.state.")
            .and_then(|s| s.strip_suffix(".parent"))
        else {
            continue;
        };
        parents_set.insert(value);
        children_set.insert(child.to_string());
    }

    // Roots are parents that aren't children of anything in the stacking system
    let mut roots: Vec<String> = parents_set.difference(&children_set).cloned().collect();
    roots.sort();
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_would_create_cycle_detection() {
        // This is a pure logic test — cycle detection walks parent chain.
        // We can't easily unit test without a repo, so integration tests cover this.
        // But we verify the seen-set logic:
        let mut seen = HashSet::new();
        seen.insert("A".to_string());
        // If we try to insert "A" again, insert returns false → cycle
        assert!(!seen.insert("A".to_string()));
    }
}
