use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Priority, Task, TaskPayload};

#[tokio::test]
async fn test_priority_enum_ordering() {
    assert!(Priority::Critical > Priority::High);
    assert!(Priority::High > Priority::Normal);
    assert!(Priority::Normal > Priority::Low);
}

#[tokio::test]
async fn test_default_priority_is_normal() {
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    assert_eq!(task.priority, Priority::Normal);
}

#[tokio::test]
async fn test_with_priority_method() {
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload).with_priority(Priority::High);
    assert_eq!(task.priority, Priority::High);
}

#[tokio::test]
async fn test_high_priority_dequeued_before_low() {
    let queue = MemoryQueue::new();

    // Enqueue low priority first
    let low_task = Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
        .with_priority(Priority::Low);

    // Then high priority
    let high_task = Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
        .with_priority(Priority::High);

    queue.enqueue(low_task).await.unwrap();
    queue.enqueue(high_task).await.unwrap();

    // High priority should be dequeued first
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "high");
    assert_eq!(dequeued.priority, Priority::High);
}

#[tokio::test]
async fn test_critical_priority_dequeued_first() {
    let queue = MemoryQueue::new();

    // Enqueue in order: Normal, Low, High, Critical
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

    queue
        .enqueue(
            Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
                .with_priority(Priority::High),
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

    // Should dequeue in priority order: Critical, High, Normal, Low
    let task1 = queue.dequeue().await.unwrap();
    assert_eq!(task1.payload.task_type, "critical");

    let task2 = queue.dequeue().await.unwrap();
    assert_eq!(task2.payload.task_type, "high");

    let task3 = queue.dequeue().await.unwrap();
    assert_eq!(task3.payload.task_type, "normal");

    let task4 = queue.dequeue().await.unwrap();
    assert_eq!(task4.payload.task_type, "low");
}

#[tokio::test]
async fn test_same_priority_maintains_fifo_order() {
    let queue = MemoryQueue::new();

    // Enqueue multiple tasks with same priority
    for i in 0..5 {
        queue
            .enqueue(
                Task::new(TaskPayload::new(
                    format!("task_{}", i),
                    serde_json::json!({}),
                ))
                .with_priority(Priority::Normal),
            )
            .await
            .unwrap();
    }

    // Should dequeue in FIFO order
    for i in 0..5 {
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.payload.task_type, format!("task_{}", i));
    }
}

#[tokio::test]
async fn test_mixed_priority_with_fifo_within_priority() {
    let queue = MemoryQueue::new();

    // Enqueue: High1, Low1, High2, Low2
    queue
        .enqueue(
            Task::new(TaskPayload::new("high1".to_string(), serde_json::json!({})))
                .with_priority(Priority::High),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("low1".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("high2".to_string(), serde_json::json!({})))
                .with_priority(Priority::High),
        )
        .await
        .unwrap();

    queue
        .enqueue(
            Task::new(TaskPayload::new("low2".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    // Should dequeue: high1, high2 (FIFO within High), low1, low2 (FIFO within Low)
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "high1");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "high2");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "low1");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "low2");
}

#[tokio::test]
async fn test_priority_with_large_queue() {
    let queue = MemoryQueue::new();

    // Enqueue 100 tasks with random priorities
    for i in 0..100 {
        let priority = match i % 4 {
            0 => Priority::Low,
            1 => Priority::Normal,
            2 => Priority::High,
            3 => Priority::Critical,
            _ => Priority::Normal,
        };

        queue
            .enqueue(
                Task::new(TaskPayload::new(
                    format!("task_{}", i),
                    serde_json::json!({"index": i}),
                ))
                .with_priority(priority),
            )
            .await
            .unwrap();
    }

    // Dequeue all and verify priority order
    let mut last_priority = Priority::Critical;
    for _ in 0..100 {
        let task = queue.dequeue().await.unwrap();
        // Current task priority should be <= last priority
        assert!(task.priority <= last_priority);
        last_priority = task.priority;
    }
}

#[tokio::test]
async fn test_backward_compatibility_default_priority() {
    let queue = MemoryQueue::new();

    // Old code without priority (uses default Normal)
    let task1 = Task::new(TaskPayload::new("task1".to_string(), serde_json::json!({})));
    let task2 = Task::new(TaskPayload::new("task2".to_string(), serde_json::json!({})));

    queue.enqueue(task1).await.unwrap();
    queue.enqueue(task2).await.unwrap();

    // Should still work in FIFO order
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "task1");
    assert_eq!(queue.dequeue().await.unwrap().payload.task_type, "task2");
}

#[tokio::test]
async fn test_priority_serialization() {
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .with_priority(Priority::High);

    // Serialize
    let json = serde_json::to_string(&task).unwrap();

    // Deserialize
    let deserialized: Task = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.priority, Priority::High);
}

#[tokio::test]
async fn test_priority_with_retries() {
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .with_priority(Priority::Critical)
        .with_max_retries(5);

    assert_eq!(task.priority, Priority::Critical);
    assert_eq!(task.max_retries, 5);
}
