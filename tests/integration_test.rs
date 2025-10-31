use task_queue_rs::{
    config::Config,
    queue::memory::MemoryQueue,
    queue::Queue,
    task::{Task, TaskPayload, TaskStatus},
};

#[tokio::test]
async fn test_end_to_end_task_flow() {
    let queue = MemoryQueue::new();

    // Create and enqueue a task
    let payload = TaskPayload::new(
        "test_task".to_string(),
        serde_json::json!({"message": "Hello, World!"}),
    );
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();
    assert_eq!(queue.size().await, 1);

    // Dequeue and verify
    let mut dequeued_task = queue.dequeue().await.unwrap();
    assert_eq!(dequeued_task.id, task_id);
    assert_eq!(dequeued_task.status, TaskStatus::Pending);

    // Simulate task execution
    dequeued_task.mark_running();
    assert_eq!(dequeued_task.status, TaskStatus::Running);

    dequeued_task.mark_completed();
    assert_eq!(dequeued_task.status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_multiple_tasks_processing() {
    let queue = MemoryQueue::new();
    let task_count = 10;

    // Enqueue multiple tasks
    for i in 0..task_count {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({"index": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    assert_eq!(queue.size().await, task_count);

    // Process all tasks
    for _ in 0..task_count {
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.status, TaskStatus::Pending);
    }

    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_task_retry_mechanism() {
    let payload = TaskPayload::new("retry_test".to_string(), serde_json::json!({}));
    let mut task = Task::with_retries(payload, 3);

    // First failure
    task.mark_failed("Error 1".to_string());
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.retry_count, 1);
    assert!(task.can_retry());

    // Second failure
    task.mark_failed("Error 2".to_string());
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.retry_count, 2);
    assert!(task.can_retry());

    // Third failure - should become dead
    task.mark_failed("Error 3".to_string());
    assert_eq!(task.status, TaskStatus::Dead);
    assert_eq!(task.retry_count, 3);
    assert!(!task.can_retry());
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = Config::default();
    assert!(config.validate().is_ok());

    config.worker_count = 0;
    assert!(config.validate().is_err());

    config.worker_count = 4;
    config.max_queue_size = 0;
    assert!(config.validate().is_err());
}
