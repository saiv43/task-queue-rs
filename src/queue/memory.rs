//! Memory

use crate::queue::Queue;
use crate::task::Task;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// In-memory queue implementation
pub struct MemoryQueue {
    tasks: Arc<RwLock<VecDeque<Task>>>,
    task_map: Arc<RwLock<HashMap<String, Task>>>,
}

impl MemoryQueue {
    /// Create a new in-memory queue
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(VecDeque::new())),
            task_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new in-memory queue with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            task_map: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
        }
    }
}

impl Default for MemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Queue for MemoryQueue {
    async fn enqueue(&self, task: Task) -> crate::Result<()> {
        let task_id = task.id.clone();

        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        tasks.push_back(task.clone());
        task_map.insert(task_id.clone(), task);

        debug!("Task {} enqueued", task_id);
        Ok(())
    }

    async fn dequeue(&self) -> crate::Result<Task> {
        let mut tasks = self.tasks.write().await;

        match tasks.pop_front() {
            Some(task) => {
                debug!("Task {} dequeued", task.id);
                Ok(task)
            }
            None => {
                warn!("Attempted to dequeue from empty queue");
                Err(crate::TaskQueueError::QueueEmpty)
            }
        }
    }

    async fn size(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.len()
    }

    async fn peek(&self) -> crate::Result<Task> {
        let tasks = self.tasks.read().await;

        match tasks.front() {
            Some(task) => Ok(task.clone()),
            None => Err(crate::TaskQueueError::QueueEmpty),
        }
    }

    async fn get(&self, task_id: &str) -> crate::Result<Task> {
        let task_map = self.task_map.read().await;

        match task_map.get(task_id) {
            Some(task) => Ok(task.clone()),
            None => Err(crate::TaskQueueError::TaskNotFound(task_id.to_string())),
        }
    }

    async fn update(&self, task: Task) -> crate::Result<()> {
        let mut task_map = self.task_map.write().await;

        if task_map.contains_key(&task.id) {
            task_map.insert(task.id.clone(), task);
            Ok(())
        } else {
            Err(crate::TaskQueueError::TaskNotFound(task.id))
        }
    }

    async fn remove(&self, task_id: &str) -> crate::Result<()> {
        let mut task_map = self.task_map.write().await;

        match task_map.remove(task_id) {
            Some(_) => {
                debug!("Task {} removed", task_id);
                Ok(())
            }
            None => Err(crate::TaskQueueError::TaskNotFound(task_id.to_string())),
        }
    }

    async fn clear(&self) -> crate::Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        tasks.clear();
        task_map.clear();

        debug!("Queue cleared");
        Ok(())
    }
}
