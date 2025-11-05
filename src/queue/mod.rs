//! In-memory queue implementation

/// Memory
pub mod memory;

use crate::task::Task;
use async_trait::async_trait;

/// Trait for queue implementations
#[async_trait]
pub trait Queue: Send + Sync {
    /// Add a task to the queue
    async fn enqueue(&self, task: Task) -> crate::Result<()>;

    /// Remove and return the next task from the queue
    async fn dequeue(&self) -> crate::Result<Task>;

    /// Get the current size of the queue
    async fn size(&self) -> usize;

    /// Check if the queue is empty
    async fn is_empty(&self) -> bool {
        self.size().await == 0
    }

    /// Peek at the next task without removing it
    async fn peek(&self) -> crate::Result<Task>;

    /// Get a task by ID
    async fn get(&self, task_id: &str) -> crate::Result<Task>;

    /// Update a task in the queue
    async fn update(&self, task: Task) -> crate::Result<()>;

    /// Remove a task by ID
    async fn remove(&self, task_id: &str) -> crate::Result<()>;

    /// Clear all tasks from the queue
    async fn clear(&self) -> crate::Result<()>;

    /// Cancel a task by ID
    async fn cancel(&self, task_id: &str) -> crate::Result<Task>;

    /// Cancel all tasks matching a predicate
    async fn cancel_where<F>(&self, predicate: F) -> crate::Result<usize>
    where
        F: Fn(&Task) -> bool + Send;

    /// Count active (non-cancelled) tasks
    async fn active_count(&self) -> usize;

    /// Count cancelled tasks
    async fn cancelled_count(&self) -> usize;

    /// Check if a task is cancelled
    async fn is_cancelled(&self, task_id: &str) -> crate::Result<bool>;
}
