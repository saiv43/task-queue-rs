//! Configuration

use serde::{Deserialize, Serialize};

/// Configuration for the task queue system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Number of worker threads
    pub worker_count: usize,

    /// Maximum number of tasks in queue
    pub max_queue_size: usize,

    /// Task timeout in seconds
    pub task_timeout_secs: u64,

    /// Maximum retry attempts for failed tasks
    pub max_retries: u32,

    /// Storage backend type
    pub storage_backend: StorageBackend,

    /// Enable task persistence
    pub enable_persistence: bool,
}

/// Storage backend types supported by the task queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageBackend {
    /// In-memory storage (non-persistent)
    Memory,
    /// Redis backend (requires Redis server)
    Redis,
    /// PostgreSQL backend (requires PostgreSQL database)
    PostgreSQL,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worker_count: num_cpus(),
            max_queue_size: 10000,
            task_timeout_secs: 300,
            max_retries: 3,
            storage_backend: StorageBackend::Memory,
            enable_persistence: false,
        }
    }
}

impl Config {
    /// Create a new configuration with custom values
    pub fn new(worker_count: usize) -> Self {
        Self {
            worker_count,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.worker_count == 0 {
            return Err(crate::TaskQueueError::ConfigError(
                "Worker count must be greater than 0".to_string(),
            ));
        }

        if self.max_queue_size == 0 {
            return Err(crate::TaskQueueError::ConfigError(
                "Max queue size must be greater than 0".to_string(),
            ));
        }

        if self.task_timeout_secs == 0 {
            return Err(crate::TaskQueueError::ConfigError(
                "Task timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
