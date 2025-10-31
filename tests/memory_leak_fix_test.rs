use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Task, TaskPayload};

#[tokio::test]
async fn test_dequeue_removes_task_from_task_map() {
    let queue = MemoryQueue::new();

    // Create and enqueue a task
    let payload = TaskPayload::new("test_task".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();

    // Verify task exists in task_map
    assert!(queue.get(&task_id).await.is_ok());

    // Dequeue the task
    queue.dequeue().await.unwrap();

    // Verify task is removed from task_map (memory leak fix)
    assert!(queue.get(&task_id).await.is_err());
}

#[tokio::test]
async fn test_multiple_dequeues_remove_all_tasks() {
    let queue = MemoryQueue::new();
    let mut task_ids = Vec::new();

    // Enqueue 10 tasks
    for i in 0..10 {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({"index": i}));
        let task = Task::new(payload);
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Verify all tasks exist
    for task_id in &task_ids {
        assert!(queue.get(task_id).await.is_ok());
    }

    // Dequeue all tasks
    for _ in 0..10 {
        queue.dequeue().await.unwrap();
    }

    // Verify all tasks are removed from task_map
    for task_id in &task_ids {
        assert!(
            queue.get(task_id).await.is_err(),
            "Task {} should be removed from task_map after dequeue",
            task_id
        );
    }
}

#[tokio::test]
async fn test_no_memory_leak_in_processing_loop() {
    let queue = MemoryQueue::new();

    // Simulate processing 100 tasks in batches
    for batch in 0..10 {
        let mut batch_task_ids = Vec::new();

        // Enqueue 10 tasks
        for i in 0..10 {
            let payload = TaskPayload::new(
                format!("batch_{}_task_{}", batch, i),
                serde_json::json!({"batch": batch, "index": i}),
            );
            let task = Task::new(payload);
            batch_task_ids.push(task.id.clone());
            queue.enqueue(task).await.unwrap();
        }

        // Process (dequeue) all 10 tasks
        for _ in 0..10 {
            queue.dequeue().await.unwrap();
        }

        // Verify all tasks from this batch are removed
        for task_id in &batch_task_ids {
            assert!(
                queue.get(task_id).await.is_err(),
                "Task {} from batch {} should not be in memory",
                task_id,
                batch
            );
        }

        // Queue should be empty
        assert_eq!(queue.size().await, 0);
    }
}

#[tokio::test]
async fn test_dequeue_and_get_interaction() {
    let queue = MemoryQueue::new();

    // Enqueue 3 tasks
    let mut task_ids = Vec::new();
    for i in 0..3 {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({}));
        let task = Task::new(payload);
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Dequeue first task
    queue.dequeue().await.unwrap();

    // First task should be gone
    assert!(queue.get(&task_ids[0]).await.is_err());

    // Other tasks should still exist
    assert!(queue.get(&task_ids[1]).await.is_ok());
    assert!(queue.get(&task_ids[2]).await.is_ok());

    // Dequeue second task
    queue.dequeue().await.unwrap();

    // First two tasks should be gone
    assert!(queue.get(&task_ids[0]).await.is_err());
    assert!(queue.get(&task_ids[1]).await.is_err());

    // Third task should still exist
    assert!(queue.get(&task_ids[2]).await.is_ok());
}

#[tokio::test]
async fn test_dequeue_empty_queue_does_not_panic() {
    let queue = MemoryQueue::new();

    // Dequeue from empty queue should return error, not panic
    let result = queue.dequeue().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_memory_cleanup_after_clear() {
    let queue = MemoryQueue::new();
    let mut task_ids = Vec::new();

    // Enqueue tasks
    for i in 0..5 {
        let payload = TaskPayload::new(format!("task_{}", i), serde_json::json!({}));
        let task = Task::new(payload);
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Clear the queue
    queue.clear().await.unwrap();

    // All tasks should be removed from task_map
    for task_id in &task_ids {
        assert!(queue.get(task_id).await.is_err());
    }
}
