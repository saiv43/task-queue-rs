/// Task executor implementations
pub mod executor;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a task in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,

    /// Task payload containing type and data
    pub payload: TaskPayload,

    /// Current status of the task
    pub status: TaskStatus,

    /// Number of retry attempts
    pub retry_count: u32,

    /// Maximum retry attempts allowed
    pub max_retries: u32,

    /// Task creation timestamp
    pub created_at: DateTime<Utc>,

    /// Task last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Task execution start time
    pub started_at: Option<DateTime<Utc>>,

    /// Task completion time
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if task failed
    pub error: Option<String>,
}

/// Task payload containing the actual work to be done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    /// Type of task (used for routing to appropriate handler)
    pub task_type: String,

    /// Task data as JSON
    pub data: serde_json::Value,
}

/// Status of a task in its lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is waiting to be processed
    Pending,

    /// Task is currently being processed
    Running,

    /// Task completed successfully
    Completed,

    /// Task failed and will be retried
    Failed,

    /// Task failed permanently (max retries exceeded)
    Dead,
}

impl Task {
    /// Create a new task with the given payload
    pub fn new(payload: TaskPayload) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            payload,
            status: TaskStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Create a new task with custom max retries
    pub fn with_retries(payload: TaskPayload, max_retries: u32) -> Self {
        let mut task = Self::new(payload);
        task.max_retries = max_retries;
        task
    }

    /// Mark task as running
    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark task as failed
    pub fn mark_failed(&mut self, error: String) {
        self.retry_count += 1;
        self.error = Some(error);
        self.updated_at = Utc::now();

        if self.retry_count >= self.max_retries {
            self.status = TaskStatus::Dead;
        } else {
            self.status = TaskStatus::Failed;
        }
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.status == TaskStatus::Failed
    }

    /// Get task age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }
}

impl TaskPayload {
    /// Create a new task payload
    pub fn new(task_type: String, data: serde_json::Value) -> Self {
        Self { task_type, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
        let task = Task::new(payload);

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert!(task.started_at.is_none());
    }

    #[test]
    fn test_task_lifecycle() {
        let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
        let mut task = Task::new(payload);

        task.mark_running();
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.started_at.is_some());

        task.mark_completed();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_retry_logic() {
        let payload = TaskPayload::new("test".to_string(), serde_json::json!({}));
        let mut task = Task::with_retries(payload, 2);

        task.mark_failed("Error 1".to_string());
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.can_retry());

        task.mark_failed("Error 2".to_string());
        assert_eq!(task.status, TaskStatus::Dead);
        assert!(!task.can_retry());
    }
}
