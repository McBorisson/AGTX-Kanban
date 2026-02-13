use agtx::db::{Task, TaskStatus};

fn create_test_task(title: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(title, "claude", "test-project");
    task.status = status;
    task
}

// === Task State Transition Tests ===

/// Valid workflow transitions:
/// Backlog → Planning → Running → Review → Done
///                        ↑__________|  (resume via 'r')

#[test]
fn test_valid_forward_transitions() {
    // Backlog can move to Planning
    let task = create_test_task("Test", TaskStatus::Backlog);
    assert!(is_valid_forward_transition(task.status, TaskStatus::Planning));

    // Planning can move to Running
    let task = create_test_task("Test", TaskStatus::Planning);
    assert!(is_valid_forward_transition(task.status, TaskStatus::Running));

    // Running can move to Review
    let task = create_test_task("Test", TaskStatus::Running);
    assert!(is_valid_forward_transition(task.status, TaskStatus::Review));

    // Review can move to Done
    let task = create_test_task("Test", TaskStatus::Review);
    assert!(is_valid_forward_transition(task.status, TaskStatus::Done));
}

#[test]
fn test_invalid_forward_transitions() {
    // Cannot skip columns
    assert!(!is_valid_forward_transition(TaskStatus::Backlog, TaskStatus::Running));
    assert!(!is_valid_forward_transition(TaskStatus::Backlog, TaskStatus::Review));
    assert!(!is_valid_forward_transition(TaskStatus::Backlog, TaskStatus::Done));
    assert!(!is_valid_forward_transition(TaskStatus::Planning, TaskStatus::Review));
    assert!(!is_valid_forward_transition(TaskStatus::Planning, TaskStatus::Done));
    assert!(!is_valid_forward_transition(TaskStatus::Running, TaskStatus::Done));
}

#[test]
fn test_done_cannot_move_forward() {
    // Done is the final state
    assert!(!is_valid_forward_transition(TaskStatus::Done, TaskStatus::Backlog));
    assert!(!is_valid_forward_transition(TaskStatus::Done, TaskStatus::Planning));
    assert!(!is_valid_forward_transition(TaskStatus::Done, TaskStatus::Running));
    assert!(!is_valid_forward_transition(TaskStatus::Done, TaskStatus::Review));
}

#[test]
fn test_review_can_resume_to_running() {
    // Review → Running is the only valid backward transition (resume)
    assert!(is_valid_resume_transition(TaskStatus::Review, TaskStatus::Running));
}

#[test]
fn test_invalid_backward_transitions() {
    // Cannot move backward except Review → Running
    assert!(!is_valid_resume_transition(TaskStatus::Done, TaskStatus::Review));
    assert!(!is_valid_resume_transition(TaskStatus::Running, TaskStatus::Planning));
    assert!(!is_valid_resume_transition(TaskStatus::Planning, TaskStatus::Backlog));
}

#[test]
fn test_next_status() {
    assert_eq!(next_status(TaskStatus::Backlog), Some(TaskStatus::Planning));
    assert_eq!(next_status(TaskStatus::Planning), Some(TaskStatus::Running));
    assert_eq!(next_status(TaskStatus::Running), Some(TaskStatus::Review));
    assert_eq!(next_status(TaskStatus::Review), Some(TaskStatus::Done));
    assert_eq!(next_status(TaskStatus::Done), None);
}

#[test]
fn test_column_indices() {
    let columns = TaskStatus::columns();

    assert_eq!(columns[0], TaskStatus::Backlog);
    assert_eq!(columns[1], TaskStatus::Planning);
    assert_eq!(columns[2], TaskStatus::Running);
    assert_eq!(columns[3], TaskStatus::Review);
    assert_eq!(columns[4], TaskStatus::Done);
}

#[test]
fn test_task_status_preserves_through_transitions() {
    let mut task = create_test_task("My Feature", TaskStatus::Backlog);

    // Simulate workflow
    task.status = TaskStatus::Planning;
    assert_eq!(task.status, TaskStatus::Planning);
    assert_eq!(task.title, "My Feature"); // Other fields preserved

    task.status = TaskStatus::Running;
    assert_eq!(task.status, TaskStatus::Running);

    task.status = TaskStatus::Review;
    assert_eq!(task.status, TaskStatus::Review);

    // Resume back to Running
    task.status = TaskStatus::Running;
    assert_eq!(task.status, TaskStatus::Running);

    task.status = TaskStatus::Review;
    task.status = TaskStatus::Done;
    assert_eq!(task.status, TaskStatus::Done);
}

// === Helper functions that mirror the app logic ===

/// Check if moving forward one step is valid
fn is_valid_forward_transition(from: TaskStatus, to: TaskStatus) -> bool {
    next_status(from) == Some(to)
}

/// Check if resume transition is valid (only Review → Running)
fn is_valid_resume_transition(from: TaskStatus, to: TaskStatus) -> bool {
    from == TaskStatus::Review && to == TaskStatus::Running
}

/// Get the next status in the workflow
fn next_status(status: TaskStatus) -> Option<TaskStatus> {
    match status {
        TaskStatus::Backlog => Some(TaskStatus::Planning),
        TaskStatus::Planning => Some(TaskStatus::Running),
        TaskStatus::Running => Some(TaskStatus::Review),
        TaskStatus::Review => Some(TaskStatus::Done),
        TaskStatus::Done => None,
    }
}
