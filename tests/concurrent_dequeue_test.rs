use std::collections::HashSet;
use std::sync::Arc;
use task_queue_rs::queue::memory::MemoryQueue;
use task_queue_rs::queue::Queue;
use task_queue_rs::task::{Priority, Task, TaskPayload};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_concurrent_dequeue_no_duplicates() {
    let queue = Arc::new(MemoryQueue::new());

    // Enqueue 100 tasks with unique IDs
    for i in 0..100 {
        let priority = match i % 4 {
            0 => Priority::Low,
            1 => Priority::Normal,
            2 => Priority::High,
            _ => Priority::Critical,
        };

        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({"id": i}),
        ))
        .with_priority(priority);

        queue.enqueue(task).await.unwrap();
    }

    assert_eq!(queue.size().await, 100);

    // Spawn 10 workers that will dequeue concurrently
    let mut handles = vec![];
    let dequeued_tasks = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    for _worker_id in 0..10 {
        let queue_clone = Arc::clone(&queue);
        let tasks_clone = Arc::clone(&dequeued_tasks);

        let handle = tokio::spawn(async move {
            let mut local_tasks = Vec::new();

            // Each worker tries to dequeue 10 tasks
            for _ in 0..10 {
                match queue_clone.dequeue().await {
                    Ok(task) => {
                        local_tasks.push(task.id.clone());
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            let mut tasks = tasks_clone.lock().await;
            tasks.extend(local_tasks);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let dequeued = dequeued_tasks.lock().await;

    // Check for duplicates using HashSet
    let unique_tasks: HashSet<_> = dequeued.iter().collect();

    assert_eq!(
        dequeued.len(),
        unique_tasks.len(),
        "Race condition detected: {} duplicate tasks found! Total dequeued: {}, Unique: {}",
        dequeued.len() - unique_tasks.len(),
        dequeued.len(),
        unique_tasks.len()
    );

    // All 100 tasks should have been dequeued exactly once
    assert_eq!(
        dequeued.len(),
        100,
        "Expected 100 tasks, got {}",
        dequeued.len()
    );

    // Queue should be empty
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_task_map_consistency_under_concurrent_load() {
    let queue = Arc::new(MemoryQueue::new());

    // Enqueue 50 tasks
    let mut task_ids = Vec::new();
    for i in 0..50 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({"id": i}),
        ));
        task_ids.push(task.id.clone());
        queue.enqueue(task).await.unwrap();
    }

    // Spawn multiple workers to dequeue concurrently
    let mut handles = vec![];
    for _ in 0..5 {
        let queue_clone = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                if queue_clone.dequeue().await.is_ok() {
                    // Small delay to increase contention
                    sleep(Duration::from_micros(1)).await;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // All dequeued tasks should be removed from task_map
    for task_id in &task_ids {
        let result = queue.get(task_id).await;
        assert!(
            result.is_err(),
            "Task {} should not exist in task_map after being dequeued",
            task_id
        );
    }

    // Queue should be empty
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_size_consistency_during_concurrent_operations() {
    let queue = Arc::new(MemoryQueue::new());

    // Enqueue 30 tasks
    for i in 0..30 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    let initial_size = queue.size().await;
    assert_eq!(initial_size, 30);

    // Dequeue all tasks concurrently
    let mut handles = vec![];
    let dequeue_count = Arc::new(tokio::sync::Mutex::new(0));

    for _ in 0..3 {
        let queue_clone = Arc::clone(&queue);
        let count_clone = Arc::clone(&dequeue_count);

        let handle = tokio::spawn(async move {
            let mut local_count = 0;
            for _ in 0..10 {
                if queue_clone.dequeue().await.is_ok() {
                    local_count += 1;
                }
            }
            let mut count = count_clone.lock().await;
            *count += local_count;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let total_dequeued = *dequeue_count.lock().await;
    let final_size = queue.size().await;

    // The sum should equal initial size
    assert_eq!(
        total_dequeued + final_size,
        initial_size,
        "Inconsistency detected: dequeued {} + remaining {} != initial {}",
        total_dequeued,
        final_size,
        initial_size
    );
}

#[tokio::test]
async fn test_high_contention_dequeue() {
    let queue = Arc::new(MemoryQueue::new());

    // Enqueue 200 tasks
    for i in 0..200 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({"index": i}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Spawn 20 workers for high contention
    let mut handles = vec![];
    let all_dequeued = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    for _ in 0..20 {
        let queue_clone = Arc::clone(&queue);
        let dequeued_clone = Arc::clone(&all_dequeued);

        let handle = tokio::spawn(async move {
            let mut local = Vec::new();
            loop {
                match queue_clone.dequeue().await {
                    Ok(task) => local.push(task.id),
                    Err(_) => break,
                }
            }

            let mut all = dequeued_clone.lock().await;
            all.extend(local);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let dequeued = all_dequeued.lock().await;

    // Verify no duplicates
    let unique: HashSet<_> = dequeued.iter().collect();
    assert_eq!(
        dequeued.len(),
        unique.len(),
        "Found {} duplicate tasks under high contention",
        dequeued.len() - unique.len()
    );

    // All 200 tasks should be dequeued
    assert_eq!(dequeued.len(), 200);
    assert_eq!(queue.size().await, 0);
}

#[tokio::test]
async fn test_concurrent_enqueue_dequeue() {
    let queue = Arc::new(MemoryQueue::new());

    // Spawn enqueuers and dequeuers simultaneously
    let mut handles = vec![];

    // 5 enqueuers adding 20 tasks each
    for worker in 0..5 {
        let queue_clone = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for i in 0..20 {
                let task = Task::new(TaskPayload::new(
                    format!("enqueuer_{}_task_{}", worker, i),
                    serde_json::json!({}),
                ));
                queue_clone.enqueue(task).await.unwrap();
                sleep(Duration::from_micros(10)).await;
            }
        });
        handles.push(handle);
    }

    // 5 dequeuers removing tasks
    let dequeued = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    for _ in 0..5 {
        let queue_clone = Arc::clone(&queue);
        let dequeued_clone = Arc::clone(&dequeued);
        let handle = tokio::spawn(async move {
            let mut local = Vec::new();
            for _ in 0..20 {
                if let Ok(task) = queue_clone.dequeue().await {
                    local.push(task.id);
                }
                sleep(Duration::from_micros(10)).await;
            }
            let mut all = dequeued_clone.lock().await;
            all.extend(local);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let dequeued_tasks = dequeued.lock().await;

    // Check no duplicates in dequeued tasks
    let unique: HashSet<_> = dequeued_tasks.iter().collect();
    assert_eq!(
        dequeued_tasks.len(),
        unique.len(),
        "Found duplicates during concurrent enqueue/dequeue"
    );
}

#[tokio::test]
async fn test_internal_consistency_verification() {
    let queue = Arc::new(MemoryQueue::new());

    // Enqueue tasks
    for i in 0..50 {
        let task = Task::new(TaskPayload::new(
            format!("task_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    // Verify consistency after enqueue
    assert!(
        queue.verify_consistency().await,
        "Queue inconsistent after enqueue"
    );

    // Concurrent operations
    let mut handles = vec![];
    for _ in 0..5 {
        let queue_clone = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for _ in 0..5 {
                if queue_clone.dequeue().await.is_ok() {
                    sleep(Duration::from_micros(1)).await;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Verify consistency after concurrent dequeue
    assert!(
        queue.verify_consistency().await,
        "Queue inconsistent after concurrent operations"
    );
}

#[tokio::test]
async fn test_stress_concurrent_operations() {
    let queue = Arc::new(MemoryQueue::new());

    // Initial load
    for i in 0..100 {
        let task = Task::new(TaskPayload::new(
            format!("initial_{}", i),
            serde_json::json!({}),
        ));
        queue.enqueue(task).await.unwrap();
    }

    let mut handles = vec![];

    // Spawn 10 workers doing mixed operations
    for worker_id in 0..10 {
        let queue_clone = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            for i in 0..20 {
                // Alternate between enqueue and dequeue
                if i % 2 == 0 {
                    let task = Task::new(TaskPayload::new(
                        format!("worker_{}_task_{}", worker_id, i),
                        serde_json::json!({}),
                    ));
                    let _ = queue_clone.enqueue(task).await;
                } else {
                    let _ = queue_clone.dequeue().await;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Final consistency check
    assert!(
        queue.verify_consistency().await,
        "Queue inconsistent after stress test"
    );
}
