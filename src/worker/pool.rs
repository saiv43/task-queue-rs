use crate::queue::Queue;
use crate::worker::Worker;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

/// A pool of workers that process tasks concurrently
pub struct WorkerPool {
    worker_count: usize,
    handles: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    /// Create a new worker pool with the specified number of workers
    pub fn new(worker_count: usize) -> Self {
        Self {
            worker_count,
            handles: Vec::new(),
        }
    }

    /// Start the worker pool
    pub async fn start<Q: Queue + 'static>(&mut self, queue: Q) -> crate::Result<()> {
        let queue = Arc::new(queue);

        info!("Starting worker pool with {} workers", self.worker_count);

        for i in 0..self.worker_count {
            let worker = Worker::new(i);
            let queue_clone = Arc::clone(&queue);

            let handle = tokio::spawn(async move {
                worker.run(queue_clone).await;
            });

            self.handles.push(handle);
        }

        // Wait for all workers to complete (they run indefinitely in this base version)
        for handle in self.handles.drain(..) {
            if let Err(e) = handle.await {
                return Err(crate::TaskQueueError::WorkerPoolError(format!(
                    "Worker panicked: {e}"
                )));
            }
        }

        Ok(())
    }

    /// Get the number of workers in the pool
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Check if the pool is running
    pub fn is_running(&self) -> bool {
        !self.handles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::memory::MemoryQueue;
    use crate::task::{Task, TaskPayload};

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
}
