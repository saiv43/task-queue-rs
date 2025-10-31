use task_queue_rs::{
    queue::memory::MemoryQueue,
    queue::Queue,
    task::{Task, TaskPayload},
};

#[tokio::test]
async fn test_queue_fifo_order() {
    let queue = MemoryQueue::new();
    let mut task_ids = Vec::new();

    // Enqueue tasks
    for i in 0..5 {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({}));
        let task = Task::new(payload);
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Dequeue and verify FIFO order
    for expected_id in task_ids {
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.id, expected_id);
    }
}

#[tokio::test]
async fn test_queue_peek() {
    let queue = MemoryQueue::new();
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();

    // Peek should not remove the task
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.id, task_id);
    assert_eq!(queue.size().await, 1);

    // Dequeue should remove it
    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.id, task_id);
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_queue_get_and_update() {
    let queue = MemoryQueue::new();
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let mut task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task.clone()).await.unwrap();

    // Get task
    let retrieved = queue.get(&task_id).await.unwrap();
    assert_eq!(retrieved.id, task_id);

    // Update task
    task.mark_running();
    queue.update(task.clone()).await.unwrap();

    let updated = queue.get(&task_id).await.unwrap();
    assert_eq!(updated.status, task_queue_rs::TaskStatus::Running);
}

#[tokio::test]
async fn test_queue_remove() {
    let queue = MemoryQueue::new();
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();
    assert_eq!(queue.size().await, 1);

    queue.remove(&task_id).await.unwrap();

    let result = queue.get(&task_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_queue_clear() {
    let queue = MemoryQueue::new();

    for i in 0..10 {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    assert_eq!(queue.size().await, 10);

    queue.clear().await.unwrap();
    assert_eq!(queue.size().await, 0);
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_concurrent_queue_operations() {
    let queue = std::sync::Arc::new(MemoryQueue::new());
    let mut handles = vec![];

    // Spawn multiple tasks to enqueue concurrently
    for i in 0..10 {
        let queue_clone = queue.clone();
        let handle = tokio::spawn(async move {
            let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({}));
            queue_clone.enqueue(Task::new(payload)).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all enqueue operations
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(queue.size().await, 10);
}
