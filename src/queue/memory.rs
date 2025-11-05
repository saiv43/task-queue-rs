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

        // Find the first ready, non-cancelled task (respecting priority order)
        let result = loop {
            match tasks.pop() {
                Some(priority_task) => {
                    // Skip cancelled tasks
                    if priority_task.task.is_cancelled() {
                        debug!("Task {} is cancelled, skipping", priority_task.task.id);
                        // Don't put back cancelled tasks, effectively removing them
                        task_map.remove(&priority_task.task.id);
                        continue;
                    }

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
        // CRITICAL: Must update BOTH heap and task_map to maintain consistency
        // Acquire both locks together to prevent race conditions
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // Check if task exists
        if !task_map.contains_key(&task.id) {
            return Err(crate::TaskQueueError::TaskNotFound(task.id));
        }

        // Get the sequence number from the old task in the heap
        // We need to maintain the original sequence for FIFO ordering within same priority
        let mut old_sequence = None;
        let mut temp_tasks = Vec::new();

        // Remove the old task from heap
        while let Some(priority_task) = tasks.pop() {
            if priority_task.task.id == task.id {
                old_sequence = Some(priority_task.sequence);
                break;
            } else {
                temp_tasks.push(priority_task);
            }
        }

        // Put back the tasks we temporarily removed
        for priority_task in temp_tasks {
            tasks.push(priority_task);
        }

        // If we found the task in the heap, re-insert with updated data
        if let Some(sequence) = old_sequence {
            tasks.push(PriorityTask {
                task: task.clone(),
                sequence,
            });
            debug!(
                "Task {} updated in heap with sequence {}",
                task.id, sequence
            );
        } else {
            // Task was in task_map but not in heap (shouldn't happen, but handle it)
            warn!(
                "Task {} found in task_map but not in heap during update",
                task.id
            );
        }

        // Update task_map
        task_map.insert(task.id.clone(), task);

        Ok(())
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

    async fn cancel(&self, task_id: &str) -> crate::Result<Task> {
        // CRITICAL: Must update BOTH heap and task_map
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;

        // Get the task
        let mut task = task_map
            .get(task_id)
            .ok_or_else(|| crate::TaskQueueError::TaskNotFound(task_id.to_string()))?
            .clone();

        // Check if task can be cancelled
        if !task.can_cancel() {
            return Err(crate::TaskQueueError::Unknown(format!(
                "Cannot cancel task in status: {:?}",
                task.status
            )));
        }

        // Mark as cancelled
        task.mark_cancelled();

        // Update in heap
        let mut old_sequence = None;
        let mut temp_tasks = Vec::new();

        while let Some(priority_task) = tasks.pop() {
            if priority_task.task.id == task_id {
                old_sequence = Some(priority_task.sequence);
                break;
            } else {
                temp_tasks.push(priority_task);
            }
        }

        // Put back non-cancelled tasks
        for priority_task in temp_tasks {
            tasks.push(priority_task);
        }

        // Re-insert cancelled task with updated status
        if let Some(sequence) = old_sequence {
            tasks.push(PriorityTask {
                task: task.clone(),
                sequence,
            });
        }

        // Update task_map
        task_map.insert(task_id.to_string(), task.clone());

        debug!("Task {} cancelled", task_id);
        Ok(task)
    }

    async fn cancel_where<F>(&self, predicate: F) -> crate::Result<usize>
    where
        F: Fn(&Task) -> bool + Send,
    {
        let mut tasks = self.tasks.write().await;
        let mut task_map = self.task_map.write().await;
        let mut cancelled_count = 0;

        // Find task IDs to cancel
        let task_ids_to_cancel: Vec<String> = task_map
            .iter()
            .filter(|(_, task)| task.can_cancel() && predicate(task))
            .map(|(id, _)| id.clone())
            .collect();

        // Update heap: mark matching tasks as cancelled
        let mut temp_tasks = Vec::new();
        while let Some(mut priority_task) = tasks.pop() {
            if task_ids_to_cancel.contains(&priority_task.task.id) {
                priority_task.task.mark_cancelled();
                cancelled_count += 1;
                debug!("Task {} cancelled by predicate", priority_task.task.id);
            }
            temp_tasks.push(priority_task);
        }

        // Put all tasks back
        for priority_task in temp_tasks {
            tasks.push(priority_task);
        }

        // Update task_map
        for task_id in &task_ids_to_cancel {
            if let Some(task) = task_map.get_mut(task_id) {
                task.mark_cancelled();
            }
        }

        Ok(cancelled_count)
    }

    async fn active_count(&self) -> usize {
        let task_map = self.task_map.read().await;
        task_map
            .values()
            .filter(|task| !task.is_cancelled())
            .count()
    }

    async fn cancelled_count(&self) -> usize {
        let task_map = self.task_map.read().await;
        task_map.values().filter(|task| task.is_cancelled()).count()
    }

    async fn is_cancelled(&self, task_id: &str) -> crate::Result<bool> {
        let task = self.get(task_id).await?;
        Ok(task.is_cancelled())
    }
}
