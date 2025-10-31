use task_queue_rs::storage::backend::MemoryBackend;
use task_queue_rs::storage::StorageBackend;
use task_queue_rs::task::{Task, TaskPayload};

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
