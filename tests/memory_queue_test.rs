use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Task, TaskPayload};

#[tokio::test]
async fn test_enqueue_dequeue() {
    let queue = MemoryQueue::new();
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();
    assert_eq!(queue.size().await, 1);

    let dequeued = queue.dequeue().await.unwrap();
    assert_eq!(dequeued.id, task_id);
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_queue_empty() {
    let queue = MemoryQueue::new();
    let result = queue.dequeue().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_task() {
    let queue = MemoryQueue::new();
    let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
    let task = Task::new(payload);
    let task_id = task.id.clone();

    queue.enqueue(task).await.unwrap();
    let retrieved = queue.get(&task_id).await.unwrap();
    assert_eq!(retrieved.id, task_id);
}

#[tokio::test]
async fn test_clear_queue() {
    let queue = MemoryQueue::new();

    for i in 0..5 {
        let payload = TaskPayload::new(format!("test_{}", i), serde_json::json!({}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    assert_eq!(queue.size().await, 5);
    queue.clear().await.unwrap();
    assert_eq!(queue.size().await, 0);
}
