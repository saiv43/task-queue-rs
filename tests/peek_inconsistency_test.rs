use chrono::Duration;
use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Priority, Task, TaskPayload};

#[tokio::test]
async fn test_peek_skips_cancelled_tasks() {
    let queue = MemoryQueue::new();

    // Enqueue high priority task
    let high = Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
        .with_priority(Priority::High);
    let high_id = high.id.clone();
    queue.enqueue(high).await.unwrap();

    // Enqueue low priority task
    queue
        .enqueue(
            Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    // Cancel the high priority task
    queue.cancel(&high_id).await.unwrap();

    // BUG FIX: peek should skip cancelled task and return low priority
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "low");
    assert!(!peeked.is_cancelled());

    // dequeue should return the same task
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "low");
    assert_eq!(peeked.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_skips_unready_scheduled_tasks() {
    let queue = MemoryQueue::new();

    // Enqueue scheduled task (not ready) with high priority
    let scheduled = Task::new(TaskPayload::new(
        "scheduled".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::High)
    .schedule_after(Duration::hours(1));
    queue.enqueue(scheduled).await.unwrap();

    // Enqueue immediate task with low priority
    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "immediate".to_string(),
                serde_json::json!({}),
            ))
            .with_priority(Priority::Low),
        )
        .await
        .unwrap();

    // BUG FIX: peek should skip unready task and return immediate
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "immediate");
    assert!(peeked.is_ready());

    // dequeue should return the same task
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "immediate");
    assert_eq!(peeked.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_returns_empty_when_all_cancelled() {
    let queue = MemoryQueue::new();

    // Enqueue tasks
    let task1 = Task::new(TaskPayload::new("task1".to_string(), serde_json::json!({})));
    let task1_id = task1.id.clone();
    let task2 = Task::new(TaskPayload::new("task2".to_string(), serde_json::json!({})));
    let task2_id = task2.id.clone();

    queue.enqueue(task1).await.unwrap();
    queue.enqueue(task2).await.unwrap();

    // Cancel all tasks
    queue.cancel(&task1_id).await.unwrap();
    queue.cancel(&task2_id).await.unwrap();

    // BUG FIX: peek should return empty
    let result = queue.peek().await;
    assert!(result.is_err());

    // dequeue should also return empty
    let dequeue_result = queue.dequeue().await;
    assert!(dequeue_result.is_err());
}

#[tokio::test]
async fn test_peek_returns_empty_when_all_scheduled() {
    let queue = MemoryQueue::new();

    // Enqueue only scheduled tasks
    for i in 0..3 {
        queue
            .enqueue(
                Task::new(TaskPayload::new(
                    format!("task_{}", i),
                    serde_json::json!({}),
                ))
                .schedule_after(Duration::hours(1)),
            )
            .await
            .unwrap();
    }

    // BUG FIX: peek should return empty (no ready tasks)
    let result = queue.peek().await;
    assert!(result.is_err());

    // dequeue should also return empty
    let dequeue_result = queue.dequeue().await;
    assert!(dequeue_result.is_err());
}

#[tokio::test]
async fn test_peek_dequeue_consistency_with_priorities() {
    let queue = MemoryQueue::new();

    // Enqueue tasks with different priorities
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

    // peek and dequeue should return same task (critical)
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "critical");

    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "critical");
    assert_eq!(peeked.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_with_mixed_cancelled_and_ready() {
    let queue = MemoryQueue::new();

    // Enqueue critical (will be cancelled)
    let critical = Task::new(TaskPayload::new(
        "critical".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::Critical);
    let critical_id = critical.id.clone();
    queue.enqueue(critical).await.unwrap();

    // Enqueue high (will be cancelled)
    let high = Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
        .with_priority(Priority::High);
    let high_id = high.id.clone();
    queue.enqueue(high).await.unwrap();

    // Enqueue normal (ready)
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

    // Cancel critical and high
    queue.cancel(&critical_id).await.unwrap();
    queue.cancel(&high_id).await.unwrap();

    // peek should skip cancelled and return normal
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "normal");
    assert_eq!(peeked.priority, Priority::Normal);

    // dequeue should return same
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "normal");
    assert_eq!(peeked.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_multiple_times_returns_same_task() {
    let queue = MemoryQueue::new();

    queue
        .enqueue(Task::new(TaskPayload::new(
            "task".to_string(),
            serde_json::json!({}),
        )))
        .await
        .unwrap();

    // Multiple peeks should return the same task
    let peek1 = queue.peek().await.unwrap();
    let peek2 = queue.peek().await.unwrap();
    let peek3 = queue.peek().await.unwrap();

    assert_eq!(peek1.id, peek2.id);
    assert_eq!(peek2.id, peek3.id);

    // And dequeue should return that same task
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(peek1.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_after_scheduled_task_becomes_ready() {
    let queue = MemoryQueue::new();

    // Enqueue scheduled task
    queue
        .enqueue(
            Task::new(TaskPayload::new(
                "scheduled".to_string(),
                serde_json::json!({}),
            ))
            .schedule_after(Duration::milliseconds(50)),
        )
        .await
        .unwrap();

    // peek should return empty (not ready yet)
    let result = queue.peek().await;
    assert!(result.is_err());

    // Wait for it to become ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now peek should return it
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.payload.task_type, "scheduled");
    assert!(peeked.is_ready());

    // dequeue should return same
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(peeked.id, dequeued.id);
}

#[tokio::test]
async fn test_peek_empty_queue() {
    let queue = MemoryQueue::new();

    // peek on empty queue should return error
    let result = queue.peek().await;
    assert!(result.is_err());
}
