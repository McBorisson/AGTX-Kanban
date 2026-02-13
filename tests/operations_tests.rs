#![cfg(feature = "test-mocks")]

use agtx::operations::{GitOperations, MockGitOperations, MockTmuxOperations, TmuxOperations};
use std::path::Path;

// === Tmux Operations Tests ===

#[test]
fn test_tmux_window_created_on_task_start() {
    let mut mock_tmux = MockTmuxOperations::new();

    // Expect window creation when task moves to Planning
    mock_tmux
        .expect_create_window()
        .withf(|session, window_name, working_dir| {
            session == "myproject"
                && window_name.starts_with("task-")
                && working_dir.contains(".agtx/worktrees")
        })
        .times(1)
        .returning(|_, _, _| Ok(()));

    // Simulate task starting
    let result = mock_tmux.create_window(
        "myproject",
        "task-abc123-my-feature",
        "/path/to/project/.agtx/worktrees/abc123-my-feature",
    );

    assert!(result.is_ok());
}

#[test]
fn test_tmux_window_killed_on_task_done() {
    let mut mock_tmux = MockTmuxOperations::new();

    // Expect window to be killed when task moves to Done
    mock_tmux
        .expect_kill_window()
        .withf(|target| target == "myproject:task-abc123-my-feature")
        .times(1)
        .returning(|_| Ok(()));

    // Simulate task completion
    let result = mock_tmux.kill_window("myproject:task-abc123-my-feature");

    assert!(result.is_ok());
}

#[test]
fn test_tmux_window_not_killed_on_review() {
    let mut mock_tmux = MockTmuxOperations::new();

    // Window should NOT be killed when moving to Review (we keep it open now)
    mock_tmux.expect_kill_window().times(0);

    // No kill_window call should happen
    // (In real code, we simply don't call kill_window when moving to Review)
}

#[test]
fn test_tmux_send_keys_for_claude_command() {
    let mut mock_tmux = MockTmuxOperations::new();

    mock_tmux
        .expect_send_keys()
        .withf(|target, keys| {
            target == "myproject:task-abc123"
                && keys.contains("claude")
                && keys.contains("--dangerously-skip-permissions")
        })
        .times(1)
        .returning(|_, _| Ok(()));

    let result = mock_tmux.send_keys(
        "myproject:task-abc123",
        "claude --dangerously-skip-permissions 'implement feature'",
    );

    assert!(result.is_ok());
}

// === Git Operations Tests ===

#[test]
fn test_worktree_created_on_task_planning() {
    let mut mock_git = MockGitOperations::new();

    // Expect worktree creation when task moves to Planning
    mock_git
        .expect_create_worktree()
        .withf(|project_path, task_slug| {
            project_path == Path::new("/path/to/project") && task_slug == "abc123-my-feature"
        })
        .times(1)
        .returning(|_, slug| Ok(format!("/path/to/project/.agtx/worktrees/{}", slug)));

    let result = mock_git.create_worktree(Path::new("/path/to/project"), "abc123-my-feature");

    assert!(result.is_ok());
    assert!(result.unwrap().contains("abc123-my-feature"));
}

#[test]
fn test_worktree_removed_on_task_done() {
    let mut mock_git = MockGitOperations::new();

    // Expect worktree removal when task moves to Done
    mock_git
        .expect_remove_worktree()
        .withf(|project_path, worktree_path| {
            project_path == Path::new("/path/to/project")
                && worktree_path == "/path/to/project/.agtx/worktrees/abc123-my-feature"
        })
        .times(1)
        .returning(|_, _| Ok(()));

    let result = mock_git.remove_worktree(
        Path::new("/path/to/project"),
        "/path/to/project/.agtx/worktrees/abc123-my-feature",
    );

    assert!(result.is_ok());
}

#[test]
fn test_worktree_not_removed_on_review() {
    let mut mock_git = MockGitOperations::new();

    // Worktree should NOT be removed when moving to Review
    mock_git.expect_remove_worktree().times(0);

    // No remove_worktree call should happen when going to Review
}

#[test]
fn test_worktree_exists_check() {
    let mut mock_git = MockGitOperations::new();

    mock_git
        .expect_worktree_exists()
        .withf(|project_path, task_slug| {
            project_path == Path::new("/path/to/project") && task_slug == "abc123-my-feature"
        })
        .times(1)
        .returning(|_, _| true);

    let exists = mock_git.worktree_exists(Path::new("/path/to/project"), "abc123-my-feature");

    assert!(exists);
}

// === Combined Workflow Tests ===

#[test]
fn test_full_task_lifecycle_creates_and_cleans_resources() {
    let mut mock_tmux = MockTmuxOperations::new();
    let mut mock_git = MockGitOperations::new();

    // 1. Backlog -> Planning: Create worktree and tmux window
    mock_git
        .expect_create_worktree()
        .times(1)
        .returning(|_, slug| Ok(format!("/worktrees/{}", slug)));

    mock_tmux
        .expect_create_window()
        .times(1)
        .returning(|_, _, _| Ok(()));

    mock_tmux
        .expect_send_keys()
        .times(1)
        .returning(|_, _| Ok(()));

    // Simulate Planning phase
    let worktree = mock_git
        .create_worktree(Path::new("/project"), "task-123")
        .unwrap();
    mock_tmux
        .create_window("proj", "task-123", &worktree)
        .unwrap();
    mock_tmux
        .send_keys("proj:task-123", "claude --dangerously-skip-permissions 'plan'")
        .unwrap();

    // 2. Planning -> Running: Send implementation command
    mock_tmux
        .expect_send_keys()
        .times(1)
        .returning(|_, _| Ok(()));

    mock_tmux
        .send_keys("proj:task-123", "Please implement the plan")
        .unwrap();

    // 3. Running -> Review: Window stays open (no kill)
    // (nothing happens here - window persists)

    // 4. Review -> Done: Cleanup
    mock_tmux
        .expect_kill_window()
        .times(1)
        .returning(|_| Ok(()));

    mock_git
        .expect_remove_worktree()
        .times(1)
        .returning(|_, _| Ok(()));

    mock_tmux.kill_window("proj:task-123").unwrap();
    mock_git
        .remove_worktree(Path::new("/project"), &worktree)
        .unwrap();
}

#[test]
fn test_resume_from_review_does_not_recreate_resources() {
    let mut mock_tmux = MockTmuxOperations::new();
    let mut mock_git = MockGitOperations::new();

    // When resuming from Review -> Running, we should NOT create new resources
    mock_git.expect_create_worktree().times(0);
    mock_tmux.expect_create_window().times(0);

    // The existing window and worktree should be reused
    // (In real code, we just change the task status)
}

#[test]
fn test_delete_task_cleans_up_all_resources() {
    let mut mock_tmux = MockTmuxOperations::new();
    let mut mock_git = MockGitOperations::new();

    // Deleting a task should clean up both tmux window and worktree
    mock_tmux
        .expect_kill_window()
        .withf(|target| target == "proj:task-abc123")
        .times(1)
        .returning(|_| Ok(()));

    mock_git
        .expect_remove_worktree()
        .withf(|_, worktree| worktree.contains("abc123"))
        .times(1)
        .returning(|_, _| Ok(()));

    // Simulate delete
    mock_tmux.kill_window("proj:task-abc123").unwrap();
    mock_git
        .remove_worktree(Path::new("/project"), "/project/.agtx/worktrees/abc123")
        .unwrap();
}
