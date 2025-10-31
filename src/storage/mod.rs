/// Backend implementations
pub mod backend;

use crate::task::Task;
use async_trait::async_trait;

/// Trait for storage backend implementations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Save a task to storage
    async fn save(&self, task: &Task) -> crate::Result<()>;

    /// Load a task from storage by ID
    async fn load(&self, task_id: &str) -> crate::Result<Task>;

    /// Delete a task from storage
    async fn delete(&self, task_id: &str) -> crate::Result<()>;

    /// List all tasks in storage
    async fn list(&self) -> crate::Result<Vec<Task>>;

    /// Update a task in storage
    async fn update(&self, task: &Task) -> crate::Result<()>;

    /// Check if storage is healthy
    async fn health_check(&self) -> bool;
}
