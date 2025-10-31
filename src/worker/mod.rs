/// Worker pool implementation
pub mod pool;

use crate::queue::Queue;
use crate::task::executor::{DefaultExecutor, TaskExecutor};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// A worker that processes tasks from a queue
pub struct Worker {
    id: usize,
    executor: Arc<dyn TaskExecutor>,
}

impl Worker {
    /// Create a new worker with the given ID
    pub fn new(id: usize) -> Self {
        Self {
            id,
            executor: Arc::new(DefaultExecutor::default()),
        }
    }

    /// Create a new worker with a custom executor
    pub fn with_executor(id: usize, executor: Arc<dyn TaskExecutor>) -> Self {
        Self { id, executor }
    }

    /// Start processing tasks from the queue
    pub async fn run<Q: Queue + 'static>(&self, queue: Arc<Q>) {
        info!("Worker {} started", self.id);

        loop {
            match queue.dequeue().await {
                Ok(mut task) => {
                    info!("Worker {} processing task {}", self.id, task.id);

                    match self.executor.execute(&mut task).await {
                        Ok(()) => {
                            info!("Worker {} completed task {}", self.id, task.id);
                            // Update task in queue
                            if let Err(e) = queue.update(task).await {
                                error!("Failed to update task: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Worker {} failed to execute task: {}", self.id, e);
                            // Update failed task in queue
                            if let Err(e) = queue.update(task).await {
                                error!("Failed to update failed task: {}", e);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Queue is empty, wait before checking again
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
}
