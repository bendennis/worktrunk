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
    assert_cmd_snapshot!(
        "set_parent",
        make_snapshot_cmd(&repo, "stack", &["set-parent", "main"], Some(&feature_path),)
    );

    // Show the tree
    assert_cmd_snapshot!(
        "show",
        make_snapshot_cmd(&repo, "stack", &["show"], Some(&feature_path),)
    );
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
    assert_cmd_snapshot!(
        "from_leaf",
        make_snapshot_cmd(&repo, "stack", &["show"], Some(&c_path),)
    );

    // Show from middle
    assert_cmd_snapshot!(
        "from_middle",
        make_snapshot_cmd(&repo, "stack", &["show"], Some(&b_path),)
    );
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
    assert_cmd_snapshot!(
        "verify",
        make_snapshot_cmd(
            &repo,
            "config",
            &["state", "parent", "get", "--branch", "feature-b"],
            None,
        )
    );
}

/// `wt list --format=json` includes parent field
#[rstest]
fn test_list_json_includes_parent(mut repo: TestRepo) {
    let feature_a_path = repo.add_worktree("feature-a");
    let feature_b_path = repo.add_worktree("feature-b");

    // Set feature-a's parent to main (default branch — should be hidden in output)
    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&feature_a_path)
        .output()
        .unwrap();

    // Set feature-b's parent to feature-a (non-default — should appear in output)
    repo.wt_command()
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&feature_b_path)
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

    // feature-a: parent is default branch (main), so it's omitted
    let feature_a = items
        .iter()
        .find(|item| item["branch"].as_str() == Some("feature-a"))
        .expect("feature-a should be in list output");
    assert!(
        feature_a.get("parent").is_none(),
        "default branch parent should be hidden"
    );

    // feature-b: parent is feature-a (non-default), so it's shown
    let feature_b = items
        .iter()
        .find(|item| item["branch"].as_str() == Some("feature-b"))
        .expect("feature-b should be in list output");
    assert_eq!(feature_b["parent"].as_str(), Some("feature-a"));

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
    repo.add_worktree("feature-a");
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
    assert!(
        stdout.trim().is_empty(),
        "Expected no parent, got: {}",
        stdout.trim()
    );
}

/// Linear cascade rebase: main → A → B, advance main, rebase from A
#[rstest]
fn test_stack_rebase_linear(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");

    // Build stack: main → A → B, each with a commit
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

    fs::write(a_path.join("a.txt"), "feature a").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add a.txt"]);

    fs::write(b_path.join("b.txt"), "feature b").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add b.txt"]);

    // Advance main
    let main_path = repo.root_path().to_path_buf();
    fs::write(main_path.join("main-update.txt"), "main update").unwrap();
    repo.run_git_in(&main_path, &["add", "main-update.txt"]);
    repo.run_git_in(&main_path, &["commit", "-m", "Advance main"]);

    // Rebase from A — should cascade to B
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&a_path),
    ));

    // Verify A has main-update.txt (rebased onto main)
    assert!(
        a_path.join("main-update.txt").exists(),
        "A should have main-update.txt after rebase"
    );

    // Verify B has both main-update.txt and a.txt (cascaded rebase)
    assert!(
        b_path.join("main-update.txt").exists(),
        "B should have main-update.txt after cascade"
    );
    assert!(
        b_path.join("a.txt").exists(),
        "B should have a.txt after cascade"
    );
}

/// Rebase when already up-to-date is a no-op
#[rstest]
fn test_stack_rebase_up_to_date(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");

    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&a_path),
    ));
}

/// Rebase with branching: A → B, A → C, verify both children rebase
#[rstest]
fn test_stack_rebase_branching(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");
    let c_path = repo.add_worktree("feature-c");

    // Build: main → A → {B, C}
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
        .args(["stack", "set-parent", "feature-a"])
        .current_dir(&c_path)
        .output()
        .unwrap();

    // Add commits to each child
    fs::write(b_path.join("b.txt"), "b content").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add b.txt"]);

    fs::write(c_path.join("c.txt"), "c content").unwrap();
    repo.run_git_in(&c_path, &["add", "c.txt"]);
    repo.run_git_in(&c_path, &["commit", "-m", "Add c.txt"]);

    // Advance A
    fs::write(a_path.join("a-update.txt"), "a update").unwrap();
    repo.run_git_in(&a_path, &["add", "a-update.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Advance A"]);

    // Rebase from A — both B and C should cascade
    let output = repo
        .wt_command()
        .args(["stack", "sync", "--no-fetch", "--no-push"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    assert!(output.status.success(), "Rebase should succeed");

    // Both children should have A's update
    assert!(
        b_path.join("a-update.txt").exists(),
        "B should have a-update.txt"
    );
    assert!(
        c_path.join("a-update.txt").exists(),
        "C should have a-update.txt"
    );
}

/// Rebase fails when child has no worktree
#[rstest]
fn test_stack_rebase_missing_worktree(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");

    // Set feature-a's parent to main so it's part of a stack
    repo.wt_command()
        .args(["stack", "set-parent", "main", "--branch", "feature-a"])
        .output()
        .unwrap();

    // Set parent for a branch that exists but has no worktree
    repo.run_git_in(repo.root_path(), &["branch", "orphan-branch"]);
    repo.wt_command()
        .args([
            "stack",
            "set-parent",
            "feature-a",
            "--branch",
            "orphan-branch",
        ])
        .output()
        .unwrap();

    // Advance A so there's something to rebase
    fs::write(a_path.join("a.txt"), "content").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "advance a"]);

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&a_path),
    ));
}

/// Rebase stops on conflict with helpful message
#[rstest]
fn test_stack_rebase_conflict(mut repo: TestRepo) {
    let a_path = repo.add_worktree("feature-a");

    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();

    // Create conflicting changes
    let main_path = repo.root_path().to_path_buf();
    fs::write(main_path.join("conflict.txt"), "main version").unwrap();
    repo.run_git_in(&main_path, &["add", "conflict.txt"]);
    repo.run_git_in(&main_path, &["commit", "-m", "Main conflict"]);

    fs::write(a_path.join("conflict.txt"), "feature version").unwrap();
    repo.run_git_in(&a_path, &["add", "conflict.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Feature conflict"]);

    // Rebase should fail with conflict info
    let output = repo
        .wt_command()
        .args(["stack", "sync", "--no-fetch", "--no-push"])
        .current_dir(&a_path)
        .output()
        .unwrap();

    assert!(!output.status.success(), "Rebase should fail on conflict");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("conflict") || stderr.contains("Rebase"),
        "Should mention conflict: {stderr}"
    );
}

/// `wt stack sync --no-fetch --no-push` is equivalent to cascade rebase
#[rstest]
fn test_stack_sync_no_fetch_no_push(mut repo: TestRepo) {
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

    fs::write(a_path.join("a.txt"), "a").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add a"]);

    fs::write(b_path.join("b.txt"), "b").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add b"]);

    // Advance main
    let main_path = repo.root_path().to_path_buf();
    fs::write(main_path.join("update.txt"), "update").unwrap();
    repo.run_git_in(&main_path, &["add", "update.txt"]);
    repo.run_git_in(&main_path, &["commit", "-m", "Advance main"]);

    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&a_path),
    ));

    // Verify cascade worked
    assert!(
        b_path.join("update.txt").exists(),
        "B should have update.txt"
    );
    assert!(b_path.join("a.txt").exists(), "B should have a.txt");
}

/// `wt stack sync` with remote: fetch, rebase, push
#[rstest]
fn test_stack_sync_with_remote(mut repo: TestRepo) {
    repo.setup_remote("main");

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

    // Add commits
    fs::write(a_path.join("a.txt"), "a").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add a"]);

    fs::write(b_path.join("b.txt"), "b").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add b"]);

    // Push branches to set up tracking
    repo.run_git_in(&a_path, &["push", "-u", "origin", "feature-a"]);
    repo.run_git_in(&b_path, &["push", "-u", "origin", "feature-b"]);

    // Advance main
    let main_path = repo.root_path().to_path_buf();
    fs::write(main_path.join("update.txt"), "update").unwrap();
    repo.run_git_in(&main_path, &["add", "update.txt"]);
    repo.run_git_in(&main_path, &["commit", "-m", "Advance main"]);
    repo.run_git_in(&main_path, &["push"]);

    // Sync (fetch + rebase + push)
    let output = repo
        .wt_command()
        .args(["stack", "sync"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Sync should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify cascade worked
    assert!(
        b_path.join("update.txt").exists(),
        "B should have update.txt after sync"
    );

    // Verify branches were pushed (check remote has our commits)
    let remote_log = std::process::Command::new("git")
        .args(["log", "--oneline", "origin/feature-a"])
        .current_dir(&main_path)
        .output()
        .unwrap();
    let remote_a = String::from_utf8_lossy(&remote_log.stdout);
    assert!(
        remote_a.contains("Add a"),
        "Remote should have feature-a commits"
    );
}

/// `wt stack sync --no-push` fetches and rebases but skips push
#[rstest]
fn test_stack_sync_no_push(mut repo: TestRepo) {
    repo.setup_remote("main");

    let a_path = repo.add_worktree("feature-a");

    repo.wt_command()
        .args(["stack", "set-parent", "main"])
        .current_dir(&a_path)
        .output()
        .unwrap();

    fs::write(a_path.join("a.txt"), "a").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add a"]);

    // Advance main and push
    let main_path = repo.root_path().to_path_buf();
    fs::write(main_path.join("update.txt"), "update").unwrap();
    repo.run_git_in(&main_path, &["add", "update.txt"]);
    repo.run_git_in(&main_path, &["commit", "-m", "Advance main"]);
    repo.run_git_in(&main_path, &["push"]);

    // Sync with --no-push
    let output = repo
        .wt_command()
        .args(["stack", "sync", "--no-push"])
        .current_dir(&a_path)
        .output()
        .unwrap();
    assert!(output.status.success(), "Sync should succeed");

    // Verify rebase happened
    assert!(
        a_path.join("update.txt").exists(),
        "A should have update.txt"
    );

    // Verify feature-a was NOT pushed (no tracking branch on remote)
    let remote_refs = std::process::Command::new("git")
        .args(["branch", "-r"])
        .current_dir(&main_path)
        .output()
        .unwrap();
    let remote_refs = String::from_utf8_lossy(&remote_refs.stdout);
    assert!(
        !remote_refs.contains("feature-a"),
        "feature-a should not be on remote"
    );
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

/// `wt stack sync` prunes branches whose upstream is gone (PR merged + branch deleted).
/// Simulates: main → A → B, A's remote ref deleted after merge, sync detects and reparents B.
#[rstest]
fn test_stack_sync_prunes_integrated_branch(mut repo: TestRepo) {
    repo.setup_remote("main");
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

    // Add a commit on A and push to create upstream tracking
    fs::write(a_path.join("a.txt"), "feature a content").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add feature a"]);
    repo.run_git_in(&a_path, &["push", "-u", "origin", "feature-a"]);

    // Add a commit on B and push
    fs::write(b_path.join("b.txt"), "feature b content").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add feature b"]);
    repo.run_git_in(&b_path, &["push", "-u", "origin", "feature-b"]);

    // Simulate A's PR being merged and remote branch deleted
    let remote_path = repo.remote_path().unwrap().to_path_buf();
    repo.run_git_in(&remote_path, &["branch", "-D", "feature-a"]);
    // Fetch --prune to make git mark feature-a's upstream as gone
    repo.run_git(&["fetch", "--prune"]);

    // Sync from B — should detect A's upstream is gone, reparent B to main
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&b_path),
    ));

    // Verify B's parent is now main (reparented from A)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-b"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "main", "B should be reparented to main");

    // Verify A has no parent (cleared)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-a"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "A should have no parent after pruning, got: {}",
        stdout.trim()
    );
}

/// `wt stack sync` prunes multiple branches whose upstreams are gone.
/// Simulates: main → A → B → C, both A and B have remote branches deleted.
#[rstest]
fn test_stack_sync_prunes_multiple_integrated(mut repo: TestRepo) {
    repo.setup_remote("main");
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");
    let c_path = repo.add_worktree("feature-c");

    // Build stack: main → A → B → C
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

    // Add commits and push each to create upstream tracking
    fs::write(a_path.join("a.txt"), "a").unwrap();
    repo.run_git_in(&a_path, &["add", "a.txt"]);
    repo.run_git_in(&a_path, &["commit", "-m", "Add a"]);
    repo.run_git_in(&a_path, &["push", "-u", "origin", "feature-a"]);

    fs::write(b_path.join("b.txt"), "b").unwrap();
    repo.run_git_in(&b_path, &["add", "b.txt"]);
    repo.run_git_in(&b_path, &["commit", "-m", "Add b"]);
    repo.run_git_in(&b_path, &["push", "-u", "origin", "feature-b"]);

    fs::write(c_path.join("c.txt"), "c").unwrap();
    repo.run_git_in(&c_path, &["add", "c.txt"]);
    repo.run_git_in(&c_path, &["commit", "-m", "Add c"]);
    repo.run_git_in(&c_path, &["push", "-u", "origin", "feature-c"]);

    // Simulate A and B merged: delete their remote branches
    let remote_path = repo.remote_path().unwrap().to_path_buf();
    repo.run_git_in(&remote_path, &["branch", "-D", "feature-a"]);
    repo.run_git_in(&remote_path, &["branch", "-D", "feature-b"]);
    repo.run_git(&["fetch", "--prune"]);

    // Sync from C — should detect A and B are gone, reparent C to main
    let settings = setup_snapshot_settings(&repo);
    let _guard = settings.bind_to_scope();
    assert_cmd_snapshot!(make_snapshot_cmd(
        &repo,
        "stack",
        &["sync", "--no-fetch", "--no-push"],
        Some(&c_path),
    ));

    // C's parent should now be main (A and B both pruned)
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-c"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "main", "C should be reparented to main");
}

/// `wt stack sync` does NOT prune a child branch at the same commit as its parent.
/// Regression test: content-based checks falsely detected same-commit as "integrated."
#[rstest]
fn test_stack_sync_no_false_positive_same_commit(mut repo: TestRepo) {
    repo.setup_remote("main");
    let a_path = repo.add_worktree("feature-a");
    let b_path = repo.add_worktree("feature-b");

    // Build stack: main → A → B (B created at same commit as A — no divergence yet)
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

    // Push both to create upstream tracking (but don't delete remote refs)
    repo.run_git_in(&a_path, &["push", "-u", "origin", "feature-a"]);
    repo.run_git_in(&b_path, &["push", "-u", "origin", "feature-b"]);

    // Sync from B — B should NOT be pruned even though it's at the same commit as A
    let output = repo
        .wt_command()
        .args(["stack", "sync", "--no-fetch", "--no-push"])
        .current_dir(&b_path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify no branches were reported as integrated
    assert!(
        !stderr.contains("integrated"),
        "No branches should be pruned, but got: {stderr}"
    );

    // Verify B's parent is still A
    let output = repo
        .wt_command()
        .args(["config", "state", "parent", "get", "--branch", "feature-b"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "feature-a",
        "B should still have A as parent, not be reparented"
    );
}
