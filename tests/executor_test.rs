use task_queue_rs::task::executor::{DefaultExecutor, TaskExecutor};
use task_queue_rs::task::{Task, TaskPayload, TaskStatus};

#[tokio::test]
async fn test_executor_success() {
    let executor = DefaultExecutor::new(5);
    let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({}));
    let mut task = Task::new(payload);

    let result = executor.execute(&mut task).await;
    assert!(result.is_ok());
    assert_eq!(task.status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_executor_different_types() {
    let executor = DefaultExecutor::new(5);

    // Test type 1
    let payload = TaskPayload::new("task_type_1".to_string(), serde_json::json!({}));
    let mut task = Task::new(payload);
    let result = executor.execute(&mut task).await;
    assert!(result.is_ok());
    assert_eq!(task.status, TaskStatus::Completed);
}
