use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Task, TaskPayload};
use task_queue_rs::worker::pool::WorkerPool;

#[tokio::test]
async fn test_worker_pool_creation() {
    let pool = WorkerPool::new(4);
    assert_eq!(pool.worker_count(), 4);
    assert!(!pool.is_running());
}

#[tokio::test]
async fn test_worker_pool_processes_tasks() {
    let queue = MemoryQueue::new();

    // Add some tasks
    for i in 0..3 {
        let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    assert_eq!(queue.size().await, 3);
}
