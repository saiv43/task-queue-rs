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

    /// Verify internal consistency between heap and map (for testing/debugging)
    /// Returns true if the queue is in a consistent state
    pub async fn verify_consistency(&self) -> bool {
        let tasks = self.tasks.read().await;
        let task_map = self.task_map.read().await;

        // Both data structures should have the same size
        tasks.len() == task_map.len()
    }

    /// Count how many tasks are ready to execute now
    pub async fn ready_count(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.iter().filter(|pt| pt.task.is_ready()).count()
    }

    /// Count how many tasks are scheduled for future execution
    pub async fn scheduled_count(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.iter().filter(|pt| !pt.task.is_ready()).count()
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

        // CRITICAL: Acquire both locks together to maintain consistency
        // This prevents race conditions where a task could be partially inserted
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // Insert into both data structures atomically
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
        // CRITICAL: Both locks must be acquired and held together to prevent race conditions
        // If these locks are acquired separately or released between operations,
        // concurrent dequeue operations could cause:
        // 1. Tasks being dequeued multiple times
        // 2. Inconsistent state between heap and map
        // 3. Memory leaks or lost tasks
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // Temporary storage for tasks that aren't ready yet
        let mut not_ready = Vec::new();

        // Find the first ready task (respecting priority order)
        let result = loop {
            match tasks.pop() {
                Some(priority_task) => {
                    // Check if task is ready to execute
                    if priority_task.task.is_ready() {
                        let task = priority_task.task;
                        // Remove from task_map atomically while holding both locks
                        task_map.remove(&task.id);
                        debug!(
                            "Task {} dequeued with priority {:?}",
                            task.id, task.priority
                        );
                        break Ok(task);
                    } else {
                        // Task not ready yet, save it to put back
                        debug!(
                            "Task {} not ready yet (scheduled for {:?})",
                            priority_task.task.id, priority_task.task.scheduled_at
                        );
                        not_ready.push(priority_task);
                    }
                }
                None => {
                    // No more tasks in queue
                    if not_ready.is_empty() {
                        warn!("Attempted to dequeue from empty queue");
                        break Err(crate::TaskQueueError::QueueEmpty);
                    } else {
                        debug!(
                            "No ready tasks available, {} scheduled tasks waiting",
                            not_ready.len()
                        );
                        break Err(crate::TaskQueueError::QueueEmpty);
                    }
                }
            }
        };

        // Put back tasks that weren't ready
        for priority_task in not_ready {
            tasks.push(priority_task);
        }

        result
        // Both locks are released here atomically
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
