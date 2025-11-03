//! Memory

use crate::queue::Queue;
use crate::task::Task;
use async_trait::async_trait;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Wrapper for Task to implement priority queue ordering
#[derive(Clone)]
struct PriorityTask {
    task: Task,
    /// Sequence number for FIFO ordering within same priority
    sequence: u64,
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority && self.sequence == other.sequence
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher priority first)
        match self.task.priority.cmp(&other.task.priority) {
            Ordering::Equal => {
                // If priorities are equal, use FIFO (lower sequence first)
                // Reverse because BinaryHeap is a max-heap
                other.sequence.cmp(&self.sequence)
            }
            other => other,
        }
    }
}

/// In-memory queue implementation using a priority queue
pub struct MemoryQueue {
    tasks: Arc<RwLock<BinaryHeap<PriorityTask>>>,
    task_map: Arc<RwLock<HashMap<String, Task>>>,
    sequence_counter: Arc<RwLock<u64>>,
}

impl MemoryQueue {
    /// Create a new in-memory queue
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(BinaryHeap::new())),
            task_map: Arc::new(RwLock::new(HashMap::new())),
            sequence_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Create a new in-memory queue with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(BinaryHeap::with_capacity(capacity))),
            task_map: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            sequence_counter: Arc::new(RwLock::new(0)),
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
        let priority = task.priority;

        // Get next sequence number for FIFO ordering within same priority
        let mut sequence_counter = self.sequence_counter.write().await;
        let sequence = *sequence_counter;
        *sequence_counter += 1;
        drop(sequence_counter);

        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // Insert into priority queue
        tasks.push(PriorityTask {
            task: task.clone(),
            sequence,
        });
        task_map.insert(task_id.clone(), task);

        debug!(
            "Task {} enqueued with priority {:?} (sequence: {})",
            task_id, priority, sequence
        );
        Ok(())
    }

    async fn dequeue(&self) -> crate::Result<Task> {
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // BinaryHeap::pop() returns the highest priority task in O(log n)
        match tasks.pop() {
            Some(priority_task) => {
                let task = priority_task.task;
                // Remove from task_map to prevent memory leak
                task_map.remove(&task.id);
                debug!(
                    "Task {} dequeued with priority {:?}",
                    task.id, task.priority
                );
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

        match tasks.peek() {
            Some(priority_task) => Ok(priority_task.task.clone()),
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
