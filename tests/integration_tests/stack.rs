//! Integration tests for `wt stack` commands.

use crate::common::{TestRepo, make_snapshot_cmd, repo, setup_snapshot_settings};
use insta::assert_snapshot;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;
use std::fs;

/// `wt stack show` with no stacked branches shows helpful message
#[rstest]
fn test_stack_show_empty(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["show"],
        Some(&feature_path),
    ));
}

/// `wt stack set-parent` sets parent and `show` renders the tree
#[rstest]
fn test_stack_set_parent_and_show(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    // Set parent
    assert_cmd_snapshot!("set_parent", make_snapshot_cmd(
        &repo,
        "stack",
        &["set-parent", "main"],
        Some(&feature_path),
    ));

    // Show the tree
    assert_cmd_snapshot!("show", make_snapshot_cmd(
        &repo,
        "stack",
        &["show"],
        Some(&feature_path),
    ));
}

/// `wt stack set-parent` rejects self-reference
#[rstest]
fn test_stack_set_parent_self_reference(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["set-parent", "feature"],
        Some(&feature_path),
    ));
}

/// `wt stack set-parent` rejects nonexistent parent
#[rstest]
fn test_stack_set_parent_nonexistent(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["set-parent", "does-not-exist"],
        Some(&feature_path),
    ));
}

/// `wt stack set-parent` detects cycles: A→B→A
#[rstest]
fn test_stack_set_parent_cycle(mut repo: TestRepo) {
    let feature_a = repo.add_worktree("feature-a");
    let feature_b = repo.add_worktree("feature-b");

    // Set A's parent to main, B's parent to A
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&feature_a)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&feature_b)
        .output()
        .unwrap();

    // Try to set A's parent to B — should fail with cycle
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["set-parent", "feature-b"],
        Some(&feature_a),
    ));
}

/// `wt stack unset-parent` removes the parent
#[rstest]
fn test_stack_unset_parent(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");

    // Set then unset
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&feature_path)
        .output()
        .unwrap();

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["unset-parent"],
        Some(&feature_path),
    ));
}

/// `wt stack unset-parent` when no parent is set
#[rstest]
fn test_stack_unset_parent_noop(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["unset-parent"],
        Some(&feature_path),
    ));
}

/// `wt stack show` with a multi-level stack: main → A → B → C
#[rstest]
fn test_stack_show_deep(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");
    let c_path = repo.add_worktree("feature-c");

    // Build the stack
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&b_path)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-b"])
        .current_dir(&c_path)
        .output()
        .unwrap();

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    // Show from leaf
    assert_cmd_snapshot!("from_leaf", make_snapshot_cmd(
        &repo,
        "stack",
        &["show"],
        Some(&c_path),
    ));

    // Show from middle
    assert_cmd_snapshot!("from_middle", make_snapshot_cmd(
        &repo,
        "stack",
        &["show"],
        Some(&b_path),
    ));
}

/// `wt stack show --all` shows multiple independent stacks
#[rstest]
fn test_stack_show_all(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");
    let x_path = repo.add_worktree("feature-x");

    // Stack 1: main → A → B
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&b_path)
        .output()
        .unwrap();

    // Stack 2: main → X (independent)
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&x_path)
        .output()
        .unwrap();

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["show", "--all"],
        Some(&a_path),
    ));
}

/// `wt stack set-parent` with `--branch` flag
#[rstest]
fn test_stack_set_parent_branch_flag(mut repo: TestRepo) {
    repo.add_worktree("feature-a");
    repo.add_worktree("feature-b");

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();

    // Set feature-b's parent to feature-a from the main worktree
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["set-parent", "feature-a", "--branch", "feature-b"],
        None,
    ));

    // Verify via config state
    assert_cmd_snapshot!("verify", make_snapshot_cmd(
        &repo,
        "config",
        &["state", "parent", "get", "--branch", "feature-b"],
        None,
    ));
}

/// `wt list --format=json` includes parent field
#[rstest]
fn test_list_json_includes_parent(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");

    // Set parent
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&feature_path)
        .output()
        .unwrap();

    let output = repo
        .wt_command()
        .args(["list", "--format=json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let items = items.as_array().unwrap();

    // Find the feature item
    let feature_item = items
        .iter()
        .find(|item| item["branch"].as_str() == Some("feature"))
        .expect("feature branch should be in list output");

    assert_eq!(feature_item["parent"].as_str(), Some("main"));

    // Main should not have a parent field
    let main_item = items
        .iter()
        .find(|item| item["branch"].as_str() == Some("main"))
        .expect("main branch should be in list output");

    assert!(main_item.get("parent").is_none());
}

/// Reparenting on merge: main → A → B, merge A into main, B reparents to main
#[rstest]
fn test_reparent_on_merge(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");

    // Build stack: main → A → B
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&b_path)
        .output()
        .unwrap();

    // Add a commit on A so merge has something to do
    fs::write(a_path.join("a.txt"), "feature a content").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add feature a"]);

    // Merge A into main
    repo.wt_command()
        .args(["merge", "--yes", "--no-verify"])
        .current_dir(&a_path)
        .output()
        .unwrap();

    // Verify B's parent is now main (reparented from A)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-b"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_snapshot!(stdout.trim(), @"main");
}

/// Reparenting on remove: main → A → B, remove A, B reparents to main
#[rstest]
fn test_reparent_on_remove(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");

    // Build stack: main → A → B
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&b_path)
        .output()
        .unwrap();

    // Remove A (force-delete since it's not merged)
    repo.wt_command()
        .args(["remove", "feature-a", "--force-delete", "--no-verify"])
        .output()
        .unwrap();

    // Verify B's parent is now main (reparented from A)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-b"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_snapshot!(stdout.trim(), @"main");
}

/// Reparenting on remove with no grandparent: A → B, remove A, B detached
#[rstest]
fn test_reparent_on_remove_detach(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");

    // Stack B on A, but A has no parent
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&b_path)
        .output()
        .unwrap();

    // Remove A
    repo.wt_command()
        .args(["remove", "feature-a", "--force-delete", "--no-verify"])
        .output()
        .unwrap();

    // B should have no parent (detached)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-b"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty(), "Expected no parent, got: {}", stdout.trim());
}

/// `wt switch -c --base` automatically sets parent
#[rstest]
fn test_switch_create_with_base_sets_parent(mut repo: TestRepo) {
    let feature_path = repo.add_worktree("feature");

    // Create child branch with --base pointing to feature
    repo.wt_command()
        .args(["switch", "--create", "child", "--base", "feature", "--yes"])
        .current_dir(&feature_path)
        .output()
        .unwrap();

    // Verify parent was set
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "child"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_snapshot!(stdout.trim(), @"feature");
}
