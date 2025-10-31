//! Task Queue RS - A high-performance distributed task queue system
//!
//! This library provides a robust task queue implementation with support for
//! multiple storage backends, concurrent task processing, and flexible configuration.

/// Configuration management for the task queue system
pub mod config;
/// Queue implementations and traits
pub mod queue;
/// Storage backend implementations
pub mod storage;
/// Task definitions and execution logic
pub mod task;
/// Worker pool and worker management
pub mod worker;

pub use config::Config;
pub use queue::memory::MemoryQueue;
pub use task::{Task, TaskPayload, TaskStatus};
pub use worker::pool::WorkerPool;

use thiserror::Error;

/// Result type for task queue operations
pub type Result<T> = std::result::Result<T, TaskQueueError>;

/// Error types for the task queue system
#[derive(Error, Debug)]
pub enum TaskQueueError {
    /// Queue is empty, no tasks available
    #[error("Queue is empty")]
    QueueEmpty,

    /// Task with the specified ID was not found
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Task execution failed with an error
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Storage backend error occurred
    #[error("Storage error: {0}")]
    StorageError(String),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Worker pool encountered an error
    #[error("Worker pool error: {0}")]
    WorkerPoolError(String),

    /// Configuration validation error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Unknown or unclassified error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let err = TaskQueueError::QueueEmpty;
        assert_eq!(err.to_string(), "Queue is empty");

        let err = TaskQueueError::TaskNotFound("test-id".to_string());
        assert_eq!(err.to_string(), "Task not found: test-id");
    }
}
