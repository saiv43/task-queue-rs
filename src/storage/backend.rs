//! Backend

use crate::storage::StorageBackend;
use crate::task::Task;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage backend implementation
pub struct MemoryBackend {
    storage: Arc<RwLock<HashMap<String, Task>>>,
}

impl MemoryBackend {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    async fn save(&self, task: &Task) -> crate::Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn load(&self, task_id: &str) -> crate::Result<Task> {
        let storage = self.storage.read().await;
        storage
            .get(task_id)
            .cloned()
            .ok_or_else(|| crate::TaskQueueError::TaskNotFound(task_id.to_string()))
    }

    async fn delete(&self, task_id: &str) -> crate::Result<()> {
        let mut storage = self.storage.write().await;
        storage
            .remove(task_id)
            .ok_or_else(|| crate::TaskQueueError::TaskNotFound(task_id.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Task>> {
        let storage = self.storage.read().await;
        Ok(storage.values().cloned().collect())
    }

    async fn update(&self, task: &Task) -> crate::Result<()> {
        let mut storage = self.storage.write().await;
        if storage.contains_key(&task.id) {
            storage.insert(task.id.clone(), task.clone());
            Ok(())
        } else {
            Err(crate::TaskQueueError::TaskNotFound(task.id.clone()))
        }
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskPayload;

    #[tokio::test]
    async fn test_memory_backend_save_load() {
        let backend = MemoryBackend::new();
        let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
        let task = Task::new(payload);
        let task_id = task.id.clone();

        backend.save(&task).await.unwrap();
        let loaded = backend.load(&task_id).await.unwrap();
        assert_eq!(loaded.id, task_id);
    }

    #[tokio::test]
    async fn test_memory_backend_delete() {
        let backend = MemoryBackend::new();
        let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
        let task = Task::new(payload);
        let task_id = task.id.clone();

        backend.save(&task).await.unwrap();
        backend.delete(&task_id).await.unwrap();

        let result = backend.load(&task_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_memory_backend_list() {
        let backend = MemoryBackend::new();

        for i in 0..5 {
            let payload = TaskPayload::new(format!("test_{}", i), serde_json::json!({}));
            backend.save(&Task::new(payload)).await.unwrap();
        }

        let tasks = backend.list().await.unwrap();
        assert_eq!(tasks.len(), 5);
    }
}
