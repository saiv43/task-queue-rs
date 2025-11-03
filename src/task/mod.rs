/// Task executor implementations
pub mod executor;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priority levels for task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    /// Low priority
    Low = 0,
    /// Normal priority
    #[default]
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Represents a task in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,

    /// Task payload containing type and data
    pub payload: TaskPayload,

    /// Task priority level
    pub priority: Priority,

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
            priority: Priority::default(),
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

    /// Set the priority of the task
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the maximum retry attempts (chainable)
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
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
