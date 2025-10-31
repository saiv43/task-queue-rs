use task_queue_rs::{
    queue::memory::MemoryQueue,
    queue::Queue,
    task::{Task, TaskPayload},
    worker::pool::WorkerPool,
};
use tokio::time::{sleep, timeout, Duration};

#[tokio::test]
async fn test_graceful_shutdown_empty_queue() {
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(2);

    // Start worker pool
    worker_pool.start(queue).await.unwrap();

    // Give workers time to start
    sleep(Duration::from_millis(100)).await;

    // Shutdown immediately
    let result = worker_pool.shutdown(Duration::from_secs(5)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_graceful_shutdown_with_tasks() {
    let queue = MemoryQueue::new();

    // Add some quick tasks
    for i in 0..5 {
        let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let mut worker_pool = WorkerPool::new(2);
    worker_pool.start(queue).await.unwrap();

    // Let workers process some tasks
    sleep(Duration::from_millis(500)).await;

    // Shutdown with reasonable timeout
    let result = worker_pool.shutdown(Duration::from_secs(10)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shutdown_timeout() {
    let queue = MemoryQueue::new();

    // Add tasks that take longer than shutdown timeout
    for i in 0..3 {
        let payload = TaskPayload::new("task_type_2".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let mut worker_pool = WorkerPool::new(1);
    worker_pool.start(queue).await.unwrap();

    // Give worker time to start processing
    sleep(Duration::from_millis(200)).await;

    // Shutdown with very short timeout (should timeout)
    let result = worker_pool.shutdown(Duration::from_millis(100)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_pool_starts_correctly() {
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(4);

    assert_eq!(worker_pool.worker_count(), 4);
    assert!(!worker_pool.is_running());

    worker_pool.start(queue).await.unwrap();
    assert!(worker_pool.is_running());
}

#[tokio::test]
async fn test_multiple_workers_process_tasks() {
    let queue = MemoryQueue::new();

    // Add multiple tasks
    for i in 0..10 {
        let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let initial_size = queue.size().await;
    assert_eq!(initial_size, 10);

    let mut worker_pool = WorkerPool::new(3);
    worker_pool.start(queue).await.unwrap();

    // Let workers process tasks
    sleep(Duration::from_millis(800)).await;

    // Shutdown
    worker_pool.shutdown(Duration::from_secs(5)).await.unwrap();
}

#[tokio::test]
async fn test_shutdown_signal_propagates_to_all_workers() {
    let queue = MemoryQueue::new();
    let worker_count = 5;
    let mut worker_pool = WorkerPool::new(worker_count);

    worker_pool.start(queue).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // All workers should stop within timeout
    let shutdown_result = timeout(
        Duration::from_secs(3),
        worker_pool.shutdown(Duration::from_secs(2)),
    )
    .await;

    assert!(shutdown_result.is_ok());
    assert!(shutdown_result.unwrap().is_ok());
}
