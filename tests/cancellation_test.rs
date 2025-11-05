use chrono::Duration;
use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Priority, Task, TaskPayload, TaskStatus};

#[tokio::test]
async fn test_cancel_pending_task() {
    let queue = MemoryQueue::new();

    // Enqueue a task
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    let task_id = task.id.clone();
    queue.enqueue(task).await.unwrap();

    // Cancel it
    let cancelled = queue.cancel(&task_id).await.unwrap();
    assert_eq!(cancelled.status, TaskStatus::Cancelled);

    // Verify it's cancelled in the queue
    let retrieved = queue.get(&task_id).await.unwrap();
    assert_eq!(retrieved.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn test_cancelled_task_not_dequeued() {
    let queue = MemoryQueue::new();

    // Enqueue two tasks
    let task1 = Task::new(TaskPayload::new("task1".to_string(), serde_json::json!({})));
    let task1_id = task1.id.clone();
    let task2 = Task::new(TaskPayload::new("task2".to_string(), serde_json::json!({})));

    queue.enqueue(task1).await.unwrap();
    queue.enqueue(task2).await.unwrap();

    // Cancel first task
    queue.cancel(&task1_id).await.unwrap();

    // Dequeue should skip cancelled task and return task2
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "task2");
    assert_ne!(dequeued.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_nonexistent_task_fails() {
    let queue = MemoryQueue::new();

    let result = queue.cancel("nonexistent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_scheduled_task() {
    let queue = MemoryQueue::new();

    // Enqueue scheduled task
    let task = Task::new(TaskPayload::new(
        "scheduled".to_string(),
        serde_json::json!({}),
    ))
    .schedule_after(Duration::hours(1));
    let task_id = task.id.clone();
    queue.enqueue(task).await.unwrap();

    // Cancel it
    let cancelled = queue.cancel(&task_id).await.unwrap();
    assert_eq!(cancelled.status, TaskStatus::Cancelled);

    // Should not be dequeued
    let result = queue.dequeue().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_where_with_predicate() {
    let queue = MemoryQueue::new();

    // Enqueue multiple tasks
    for i in 0..10 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({"index": i}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Cancel tasks where index is even
    let cancelled_count = queue
        .cancel_where(|task| task.payload.data["index"].as_i64().unwrap() % 2 == 0)
        .await
        .unwrap();

    assert_eq!(cancelled_count, 5);

    // Dequeue should only get odd-indexed tasks
    for _ in 0..5 {
        let task = queue.dequeue().await.unwrap();
        let index = task.payload.data["index"].as_i64().unwrap();
        assert_eq!(index % 2, 1);
    }

    // No more tasks
    assert!(queue.dequeue().await.is_err());
}

#[tokio::test]
async fn test_cancel_by_task_type() {
    let queue = MemoryQueue::new();

    // Enqueue different task types
    queue
        .enqueue(Task::new(TaskPayload::new(
            "email".to_string(),
            serde_json::json!({}),
        )))
        .await
        .unwrap();
    queue
        .enqueue(Task::new(TaskPayload::new(
            "sms".to_string(),
            serde_json::json!({}),
        )))
        .await
        .unwrap();
    queue
        .enqueue(Task::new(TaskPayload::new(
            "email".to_string(),
            serde_json::json!({}),
        )))
        .await
        .unwrap();

    // Cancel all email tasks
    let cancelled = queue
        .cancel_where(|task| task.payload.task_type == "email")
        .await
        .unwrap();

    assert_eq!(cancelled, 2);

    // Only SMS task should remain
    let task = queue.dequeue().await.unwrap();
    assert_eq!(task.payload.task_type, "sms");
}

#[tokio::test]
async fn test_cancel_by_priority() {
    let queue = MemoryQueue::new();

    // Enqueue tasks with different priorities
    queue
        .enqueue(
            Task::new(TaskPayload::new("low1".to_string(), serde_json::json!({})))
                .with_priority(Priority::Low),
        )
        .await
        .unwrap();
    queue
        .enqueue(
            Task::new(TaskPayload::new("high1".to_string(), serde_json::json!({})))
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

    // Cancel all low priority tasks
    let cancelled = queue
        .cancel_where(|task| task.priority == Priority::Low)
        .await
        .unwrap();

    assert_eq!(cancelled, 2);

    // Only high priority task remains
    let task = queue.dequeue().await.unwrap();
    assert_eq!(task.payload.task_type, "high1");
}

#[tokio::test]
async fn test_active_count_and_cancelled_count() {
    let queue = MemoryQueue::new();

    // Enqueue 10 tasks
    let mut task_ids = Vec::new();
    for i in 0..10 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    assert_eq!(queue.size().await, 10);
    assert_eq!(queue.active_count().await, 10);
    assert_eq!(queue.cancelled_count().await, 0);

    // Cancel 3 tasks
    for i in 0..3 {
        queue.cancel(&task_ids[i]).await.unwrap();
    }

    assert_eq!(queue.size().await, 10); // Size includes cancelled
    assert_eq!(queue.active_count().await, 7);
    assert_eq!(queue.cancelled_count().await, 3);
}

#[tokio::test]
async fn test_is_cancelled_method() {
    let queue = MemoryQueue::new();

    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    let task_id = task.id.clone();
    queue.enqueue(task).await.unwrap();

    // Initially not cancelled
    assert!(!queue.is_cancelled(&task_id).await.unwrap());

    // Cancel it
    queue.cancel(&task_id).await.unwrap();

    // Now it's cancelled
    assert!(queue.is_cancelled(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_cancel_all_tasks() {
    let queue = MemoryQueue::new();

    // Enqueue multiple tasks
    for i in 0..20 {
        queue
            .enqueue(Task::new(TaskPayload::new(
                format!("task_{}", i),
                serde_json::json!({}),
            )))
            .await
            .unwrap();
    }

    // Cancel all tasks
    let cancelled = queue.cancel_where(|_| true).await.unwrap();
    assert_eq!(cancelled, 20);

    // Cancelled tasks remain in queue until dequeue attempts to process them
    assert_eq!(queue.cancelled_count().await, 20);
    assert_eq!(queue.active_count().await, 0);

    // No tasks should be dequeued (all are cancelled and will be removed)
    assert!(queue.dequeue().await.is_err());

    // After dequeue attempt, cancelled tasks are cleaned up
    assert_eq!(queue.cancelled_count().await, 0);
}

#[tokio::test]
async fn test_cancel_old_tasks() {
    let queue = MemoryQueue::new();

    // Enqueue tasks
    for i in 0..5 {
        queue
            .enqueue(Task::new(TaskPayload::new(
                format!("task_{}", i),
                serde_json::json!({}),
            )))
            .await
            .unwrap();
    }

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cancel tasks older than 50ms (should be all of them)
    let cancelled = queue
        .cancel_where(|task| task.age_seconds() >= 0)
        .await
        .unwrap();

    assert_eq!(cancelled, 5);
}

#[tokio::test]
async fn test_cancelled_task_removed_on_dequeue() {
    let queue = MemoryQueue::new();

    // Enqueue and cancel a task
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    let task_id = task.id.clone();
    queue.enqueue(task).await.unwrap();
    queue.cancel(&task_id).await.unwrap();

    // Enqueue another task
    queue
        .enqueue(Task::new(TaskPayload::new(
            "test2".to_string(),
            serde_json::json!({}),
        )))
        .await
        .unwrap();

    // Dequeue should skip cancelled task
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "test2");

    // Cancelled task should be removed from queue
    let result = queue.get(&task_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_with_mixed_priorities() {
    let queue = MemoryQueue::new();

    // Enqueue tasks with different priorities
    let high = Task::new(TaskPayload::new("high".to_string(), serde_json::json!({})))
        .with_priority(Priority::High);
    let high_id = high.id.clone();

    let low = Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
        .with_priority(Priority::Low);

    let critical = Task::new(TaskPayload::new(
        "critical".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::Critical);

    queue.enqueue(high).await.unwrap();
    queue.enqueue(low).await.unwrap();
    queue.enqueue(critical).await.unwrap();

    // Cancel high priority task
    queue.cancel(&high_id).await.unwrap();

    // Should dequeue critical first
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.payload.task_type, "critical");

    // Then low (high was cancelled)
    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.payload.task_type, "low");
}

#[tokio::test]
async fn test_concurrent_cancellation() {
    use std::sync::Arc;

    let queue = Arc::new(MemoryQueue::new());

    // Enqueue many tasks
    let mut task_ids = Vec::new();
    for i in 0..100 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Spawn multiple tasks to cancel concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let q = Arc::clone(&queue);
        let ids = task_ids.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let idx = i * 10 + j;
                let _ = q.cancel(&ids[idx]).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // All tasks should be cancelled
    assert_eq!(queue.cancelled_count().await, 100);
    assert_eq!(queue.active_count().await, 0);
}
