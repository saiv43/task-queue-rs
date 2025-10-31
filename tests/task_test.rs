use task_queue_rs::task::{Task, TaskPayload, TaskStatus};

#[test]
fn test_task_creation() {
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);

    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.retry_count, 0);
    assert!(task.started_at.is_none());
}

#[test]
fn test_task_lifecycle() {
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let mut task = Task::new(payload);

    task.mark_running();
    assert_eq!(task.status, TaskStatus::Running);
    assert!(task.started_at.is_some());

    task.mark_completed();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.completed_at.is_some());
}

#[test]
fn test_task_retry_logic() {
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let mut task = Task::with_retries(payload, 2);

    task.mark_failed("Error 1".to_string());
    assert_eq!(task.status, TaskStatus::Failed);
    assert!(task.can_retry());

    task.mark_failed("Error 2".to_string());
    assert_eq!(task.status, TaskStatus::Dead);
    assert!(!task.can_retry());
}
