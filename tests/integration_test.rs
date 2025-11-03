use task_queue_rs::{
    config::Config,
    queue::memory::MemoryQueue,
    queue::Queue,
    task::{Priority, Task, TaskPayload, TaskStatus},
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

#[tokio::test]
async fn test_priority_queue_ordering() {
    let queue = MemoryQueue::new();

    // Enqueue tasks in mixed priority order
    let low_task = Task::new(TaskPayload::new(
        "low_priority".to_string(),
        serde_json::json!({"priority": "low"}),
    ))
    .with_priority(Priority::Low);

    let normal_task = Task::new(TaskPayload::new(
        "normal_priority".to_string(),
        serde_json::json!({"priority": "normal"}),
    ))
    .with_priority(Priority::Normal);

    let high_task = Task::new(TaskPayload::new(
        "high_priority".to_string(),
        serde_json::json!({"priority": "high"}),
    ))
    .with_priority(Priority::High);

    let critical_task = Task::new(TaskPayload::new(
        "critical_priority".to_string(),
        serde_json::json!({"priority": "critical"}),
    ))
    .with_priority(Priority::Critical);

    // Enqueue in non-priority order: low, critical, normal, high
    queue.enqueue(low_task).await.unwrap();
    queue.enqueue(critical_task).await.unwrap();
    queue.enqueue(normal_task).await.unwrap();
    queue.enqueue(high_task).await.unwrap();

    assert_eq!(queue.size().await, 4);

    // Dequeue should return in priority order: Critical, High, Normal, Low
    let task1 = queue.dequeue().await.unwrap();
    assert_eq!(task1.priority, Priority::Critical);
    assert_eq!(task1.payload.task_type, "critical_priority");

    let task2 = queue.dequeue().await.unwrap();
    assert_eq!(task2.priority, Priority::High);
    assert_eq!(task2.payload.task_type, "high_priority");

    let task3 = queue.dequeue().await.unwrap();
    assert_eq!(task3.priority, Priority::Normal);
    assert_eq!(task3.payload.task_type, "normal_priority");

    let task4 = queue.dequeue().await.unwrap();
    assert_eq!(task4.priority, Priority::Low);
    assert_eq!(task4.payload.task_type, "low_priority");

    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_priority_queue_fifo_within_same_priority() {
    let queue = MemoryQueue::new();

    // Enqueue multiple tasks with the same priority
    for i in 0..5 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({"index": i}),
        ))
        .with_priority(Priority::High);
        queue.enqueue(task).await.unwrap();
    }

    // Should dequeue in FIFO order (0, 1, 2, 3, 4)
    for i in 0..5 {
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.payload.task_type, format!("task_{}", i));
        assert_eq!(task.priority, Priority::High);
    }
}

#[tokio::test]
async fn test_priority_queue_mixed_priorities_fifo() {
    let queue = MemoryQueue::new();

    // Enqueue: High, Low, High, Low, Critical
    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "high_1".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::High),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("low_1".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "high_2".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::High),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("low_2".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "critical_1".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::Critical),
        )
        .await
        .unwrap();

    // Expected dequeue order: critical_1, high_1, high_2, low_1, low_2
    assert_eq!(
        queue.dequeue().await.unwrap().payload.task_type,
        "critical_1"
    );
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "high_1");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "high_2");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "low_1");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "low_2");
}

#[tokio::test]
async fn test_peek_returns_highest_priority() {
    let queue = MemoryQueue::new();

    queue
        .enqueue(
            Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "critical".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::Critical),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "normal".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::Normal),
        )
        .await
        .unwrap();

    // Peek should return critical without removing it
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "critical");
    assert_eq!(peeked.priority, Priority::Critical);
    assert_eq!(queue.size().await, 3); // Size unchanged

    // Dequeue should also return critical
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "critical");
    assert_eq!(queue.size().await, 2);
}
