use chrono::{Duration, Utc};
use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Task, TaskPayload};
use tokio::time::sleep;

#[tokio::test]
async fn test_schedule_at_specific_time() {
    let queue = MemoryQueue::new();

    // Schedule task for 100ms in the future
    let scheduled_time = Utc::now() + Duration::milliseconds(100);
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_at(scheduled_time);

    assert!(!task.is_ready());
    assert_eq!(task.scheduled_at, Some(scheduled_time));

    queue.enqueue(task).await.unwrap();
    assert_eq!(queue.size().await, 1);

    // Try to dequeue immediately - should fail (task not ready)
    let result = queue.dequeue().await;
    assert!(result.is_err());
    assert_eq!(queue.size().await, 1);

    // Wait for scheduled time
    sleep(tokio::time::Duration::from_millis(150)).await;

    // Now should be able to dequeue
    let dequeued = queue.dequeue().await.unwrap();
    assert!(dequeued.is_ready());
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_schedule_after_delay() {
    let queue = MemoryQueue::new();

    // Schedule task for 50ms delay
    let task = Task::new(TaskPayload::new(
        "delayed".to_string(),
        serde_json::json!({}),
    ))
    .schedule_after(Duration::milliseconds(50));

    assert!(!task.is_ready());
    assert!(task.scheduled_at.is_some());

    queue.enqueue(task).await.unwrap();

    // Should not be ready immediately
    let result = queue.dequeue().await;
    assert!(result.is_err());

    // Wait for delay
    sleep(tokio::time::Duration::from_millis(100)).await;

    // Should be ready now
    let dequeued = queue.dequeue().await.unwrap();
    assert!(dequeued.is_ready());
}

#[tokio::test]
async fn test_immediate_task_without_scheduling() {
    let queue = MemoryQueue::new();

    // Task without scheduling should be ready immediately
    let task = Task::new(TaskPayload::new(
        "immediate".to_string(),
        serde_json::json!({}),
    ));

    assert!(task.is_ready());
    assert!(task.scheduled_at.is_none());

    queue.enqueue(task).await.unwrap();

    // Should be able to dequeue immediately
    let dequeued = queue.dequeue().await.unwrap();
    assert!(dequeued.is_ready());
}

#[tokio::test]
async fn test_mixed_immediate_and_scheduled_tasks() {
    let queue = MemoryQueue::new();

    // Enqueue immediate task
    let immediate = Task::new(TaskPayload::new(
        "immediate".to_string(),
        serde_json::json!({}),
    ));
    queue.enqueue(immediate).await.unwrap();

    // Enqueue scheduled task
    let scheduled = Task::new(TaskPayload::new(
        "scheduled".to_string(),
        serde_json::json!({}),
    ))
    .schedule_after(Duration::milliseconds(100));
    queue.enqueue(scheduled).await.unwrap();

    assert_eq!(queue.size().await, 2);

    // Should dequeue immediate task first
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.payload.task_type, "immediate");
    assert_eq!(queue.size().await, 1);

    // Scheduled task should still be in queue but not ready
    let result = queue.dequeue().await;
    assert!(result.is_err());
    assert_eq!(queue.size().await, 1);

    // Wait for scheduled task
    sleep(tokio::time::Duration::from_millis(150)).await;

    // Now scheduled task should be ready
    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.payload.task_type, "scheduled");
}

#[tokio::test]
async fn test_priority_with_scheduling() {
    let queue = MemoryQueue::new();

    // High priority scheduled task
    let high_scheduled = Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
        .with_priority(task_queue_rs::Priority::High)
        .schedule_after(Duration::milliseconds(50));

    // Low priority immediate task
    let low_immediate = Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
        .with_priority(task_queue_rs::Priority::Low);

    queue.enqueue(high_scheduled).await.unwrap();
    queue.enqueue(low_immediate).await.unwrap();

    // Low priority immediate task should be dequeued first (high priority not ready)
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.payload.task_type, "low");

    // Wait for high priority task
    sleep(tokio::time::Duration::from_millis(100)).await;

    // High priority task should be dequeued now
    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.payload.task_type, "high");
}

#[tokio::test]
async fn test_scheduled_in_past_executes_immediately() {
    let queue = MemoryQueue::new();

    // Schedule task in the past
    let past_time = Utc::now() - Duration::seconds(10);
    let task = Task::new(TaskPayload::new("past".to_string(), serde_json::json!({})))
        .schedule_at(past_time);

    // Should be ready immediately
    assert!(task.is_ready());

    queue.enqueue(task).await.unwrap();

    // Should be able to dequeue immediately
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "past");
}

#[tokio::test]
async fn test_is_ready_method() {
    // Immediate task
    let immediate = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    assert!(immediate.is_ready());

    // Future task
    let future = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_after(Duration::hours(1));
    assert!(!future.is_ready());

    // Past task
    let past = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_at(Utc::now() - Duration::minutes(5));
    assert!(past.is_ready());
}

#[tokio::test]
async fn test_time_until_ready() {
    // Immediate task
    let immediate = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    assert!(immediate.time_until_ready().is_none());

    // Future task
    let delay = Duration::seconds(60);
    let future = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_after(delay);

    let time_until = future.time_until_ready();
    assert!(time_until.is_some());
    let remaining = time_until.unwrap();
    // Should be approximately 60 seconds (with small tolerance)
    assert!(remaining.num_seconds() >= 59 && remaining.num_seconds() <= 61);

    // Past task
    let past = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_at(Utc::now() - Duration::minutes(5));
    assert!(past.time_until_ready().is_none());
}

#[tokio::test]
async fn test_ready_count_and_scheduled_count() {
    let queue = MemoryQueue::new();

    // Add 3 immediate tasks
    for i in 0..3 {
        let task = Task::new(TaskPayload::new(
            format!("immediate_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Add 2 scheduled tasks
    for i in 0..2 {
        let task = Task::new(TaskPayload::new(
            format!("scheduled_{}", i),
            serde_json::json!({}),
        ))
        .schedule_after(Duration::milliseconds(100));
        queue.enqueue(task).await.unwrap();
    }

    assert_eq!(queue.size().await, 5);
    assert_eq!(queue.ready_count().await, 3);
    assert_eq!(queue.scheduled_count().await, 2);

    // Dequeue immediate tasks
    for _ in 0..3 {
        queue.dequeue().await.unwrap();
    }

    assert_eq!(queue.size().await, 2);
    assert_eq!(queue.ready_count().await, 0);
    assert_eq!(queue.scheduled_count().await, 2);

    // Wait for scheduled tasks
    sleep(tokio::time::Duration::from_millis(150)).await;

    assert_eq!(queue.ready_count().await, 2);
    assert_eq!(queue.scheduled_count().await, 0);
}

#[tokio::test]
async fn test_multiple_scheduled_tasks_different_times() {
    let queue = MemoryQueue::new();

    // Schedule 3 tasks at different times
    let task1 = Task::new(TaskPayload::new("task1".to_string(), serde_json::json!({})))
        .schedule_after(Duration::milliseconds(50));
    let task2 = Task::new(TaskPayload::new("task2".to_string(), serde_json::json!({})))
        .schedule_after(Duration::milliseconds(100));
    let task3 = Task::new(TaskPayload::new("task3".to_string(), serde_json::json!({})))
        .schedule_after(Duration::milliseconds(150));

    queue.enqueue(task3).await.unwrap();
    queue.enqueue(task1).await.unwrap();
    queue.enqueue(task2).await.unwrap();

    assert_eq!(queue.size().await, 3);

    // Wait for first task
    sleep(tokio::time::Duration::from_millis(60)).await;
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.payload.task_type, "task1");
    assert_eq!(queue.size().await, 2);

    // Wait for second task
    sleep(tokio::time::Duration::from_millis(50)).await;
    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.payload.task_type, "task2");
    assert_eq!(queue.size().await, 1);

    // Wait for third task
    sleep(tokio::time::Duration::from_millis(50)).await;
    let third = queue.dequeue().await.unwrap();
    assert_eq!(third.payload.task_type, "task3");
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    let queue = MemoryQueue::new();

    // Simulate retry with exponential backoff
    let retry_count = 2u32;
    let base_delay = Duration::milliseconds(50);
    let backoff_delay = base_delay * 2_i32.pow(retry_count);

    let retry_task = Task::new(TaskPayload::new("retry".to_string(), serde_json::json!({})))
        .schedule_after(backoff_delay);

    assert!(!retry_task.is_ready());
    queue.enqueue(retry_task).await.unwrap();

    // Should not be ready immediately
    assert!(queue.dequeue().await.is_err());

    // Wait for backoff period (50ms * 2^2 = 200ms)
    sleep(tokio::time::Duration::from_millis(250)).await;

    // Should be ready now
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "retry");
}

#[tokio::test]
async fn test_scheduled_task_serialization() {
    let scheduled_time = Utc::now() + Duration::hours(1);
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_at(scheduled_time);

    // Serialize
    let json = serde_json::to_string(&task).unwrap();

    // Deserialize
    let deserialized: Task = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.scheduled_at, Some(scheduled_time));
    assert_eq!(deserialized.payload.task_type, "test");
}

#[tokio::test]
async fn test_backward_compatibility_no_scheduling() {
    let queue = MemoryQueue::new();

    // Old-style task without scheduling
    let task = Task::new(TaskPayload::new(
        "legacy".to_string(),
        serde_json::json!({}),
    ));

    assert!(task.scheduled_at.is_none());
    assert!(task.is_ready());

    queue.enqueue(task).await.unwrap();

    // Should work exactly as before
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "legacy");
}
