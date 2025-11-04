use chrono::{Duration, Utc};
use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Priority, Task, TaskPayload};

#[tokio::test]
async fn test_update_scheduled_time_causes_inconsistency() {
    let queue = MemoryQueue::new();

    // Enqueue an immediate task
    let mut task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    let task_id = task.id.clone();
    queue.enqueue(task.clone()).await.unwrap();

    // Task should be ready
    assert_eq!(queue.ready_count().await, 1);

    // Update: Schedule it for 1 hour later
    task.scheduled_at = Some(Utc::now() + Duration::hours(1));
    queue.update(task).await.unwrap();

    // After update, task_map should show it's scheduled
    let from_map = queue.get(&task_id).await.unwrap();
    assert!(from_map.scheduled_at.is_some());

    // BUG FIX: Should NOT be able to dequeue (it's scheduled for future)
    let result = queue.dequeue().await;
    assert!(
        result.is_err(),
        "Should not be able to dequeue a scheduled task"
    );

    // Should have 0 ready tasks
    assert_eq!(queue.ready_count().await, 0);
    assert_eq!(queue.scheduled_count().await, 1);
}

#[tokio::test]
async fn test_update_priority_maintains_correct_order() {
    let queue = MemoryQueue::new();

    // Enqueue low priority task
    let mut low_task = Task::new(TaskPayload::new("low".to_string(), serde_json::json!({})))
        .with_priority(Priority::Low);
    let _low_id = low_task.id.clone();

    // Enqueue normal priority task
    let normal_task = Task::new(TaskPayload::new(
        "normal".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::Normal);

    queue.enqueue(low_task.clone()).await.unwrap();
    queue.enqueue(normal_task).await.unwrap();

    // Normal should be dequeued first (higher priority)
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.payload.task_type, "normal");

    // Now update low task to Critical priority
    low_task.priority = Priority::Critical;
    queue.update(low_task).await.unwrap();

    // Re-enqueue the normal task for comparison
    let another_normal = Task::new(TaskPayload::new(
        "normal2".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::Normal);
    queue.enqueue(another_normal).await.unwrap();

    // BUG FIX: Updated Critical task should be dequeued before Normal
    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.payload.task_type, "low");
    assert_eq!(second.priority, Priority::Critical);
}

#[tokio::test]
async fn test_update_payload_reflects_in_dequeue() {
    let queue = MemoryQueue::new();

    // Enqueue task with original payload
    let mut task = Task::new(TaskPayload::new(
        "original".to_string(),
        serde_json::json!({"value": 1}),
    ));
    let _task_id = task.id.clone();
    queue.enqueue(task.clone()).await.unwrap();

    // Update payload
    task.payload = TaskPayload::new("modified".to_string(), serde_json::json!({"value": 2}));
    queue.update(task).await.unwrap();

    // BUG FIX: Dequeued task should have updated payload
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.payload.task_type, "modified");
    assert_eq!(dequeued.payload.data["value"], 2);
}

#[tokio::test]
async fn test_update_from_scheduled_to_immediate() {
    let queue = MemoryQueue::new();

    // Enqueue scheduled task
    let mut task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})))
        .schedule_after(Duration::hours(1));
    let _task_id = task.id.clone();
    queue.enqueue(task.clone()).await.unwrap();

    // Should not be ready
    assert_eq!(queue.ready_count().await, 0);
    assert!(queue.dequeue().await.is_err());

    // Update: Make it immediate
    task.scheduled_at = None;
    queue.update(task).await.unwrap();

    // BUG FIX: Should now be ready to dequeue
    assert_eq!(queue.ready_count().await, 1);
    let dequeued = queue.dequeue().await.unwrap();
    assert!(dequeued.scheduled_at.is_none());
}

#[tokio::test]
async fn test_update_multiple_fields_simultaneously() {
    let queue = MemoryQueue::new();

    // Enqueue task
    let mut task = Task::new(TaskPayload::new(
        "original".to_string(),
        serde_json::json!({}),
    ))
    .with_priority(Priority::Low);
    let task_id = task.id.clone();
    queue.enqueue(task.clone()).await.unwrap();

    // Update multiple fields
    task.priority = Priority::Critical;
    task.payload.task_type = "modified".to_string();
    task.scheduled_at = Some(Utc::now() + Duration::hours(1));
    queue.update(task).await.unwrap();

    // Verify task_map has updates
    let from_map = queue.get(&task_id).await.unwrap();
    assert_eq!(from_map.priority, Priority::Critical);
    assert_eq!(from_map.payload.task_type, "modified");
    assert!(from_map.scheduled_at.is_some());

    // BUG FIX: Should not be able to dequeue (scheduled)
    assert!(queue.dequeue().await.is_err());
}

#[tokio::test]
async fn test_update_nonexistent_task_fails() {
    let queue = MemoryQueue::new();

    // Create task but don't enqueue it
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));

    // Update should fail
    let result = queue.update(task).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_after_dequeue_fails() {
    let queue = MemoryQueue::new();

    // Enqueue and dequeue
    let task = Task::new(TaskPayload::new("test".to_string(), serde_json::json!({})));
    let _task_id = task.id.clone();
    queue.enqueue(task.clone()).await.unwrap();
    queue.dequeue().await.unwrap();

    // Update should fail (task no longer in queue)
    let result = queue.update(task).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_consistency_maintained_after_update() {
    let queue = MemoryQueue::new();

    // Enqueue multiple tasks
    for i in 0..10 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Get one task and update it
    let mut task = queue.peek().await.unwrap();
    task.priority = Priority::Critical;
    queue.update(task).await.unwrap();

    // Consistency should be maintained
    assert!(queue.verify_consistency().await);
}

#[tokio::test]
async fn test_update_with_concurrent_operations() {
    use std::sync::Arc;

    let queue = Arc::new(MemoryQueue::new());

    // Enqueue tasks
    for i in 0..20 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Spawn tasks that update and dequeue concurrently
    let mut handles = vec![];

    // Updater
    for _ in 0..5 {
        let q = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                if let Ok(mut task) = q.peek().await {
                    task.priority = Priority::High;
                    let _ = q.update(task).await;
                }
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
        });
        handles.push(handle);
    }

    // Dequeuer
    for _ in 0..5 {
        let q = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for _ in 0..4 {
                let _ = q.dequeue().await;
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Consistency should be maintained
    assert!(queue.verify_consistency().await);
}
