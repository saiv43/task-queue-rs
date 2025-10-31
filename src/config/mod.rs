//! Configuration

use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use tracing::{info, warn};

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

    /// Graceful shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,
}

/// Storage backend types supported by the task queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// In-memory storage (non-persistent)
    Memory,
    /// Redis backend (requires Redis server)
    Redis,
    /// PostgreSQL backend (requires PostgreSQL database)
    #[serde(alias = "postgres")]
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
            shutdown_timeout_secs: 30,
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

    /// Load configuration from file, environment variables, or defaults
    pub fn load() -> crate::Result<Self> {
        // Try to load from config file specified in environment variable
        if let Ok(config_path) = env::var("TASK_QUEUE_CONFIG") {
            info!("Loading config from TASK_QUEUE_CONFIG: {}", config_path);
            return Self::from_file(&config_path);
        }

        // Try default config file locations
        let default_paths = vec![
            "config.yaml",
            "config.toml",
            "config/config.yaml",
            "config/config.toml",
        ];

        for path in default_paths {
            if Path::new(path).exists() {
                info!("Loading config from: {}", path);
                return Self::from_file(path);
            }
        }

        // Try environment variables
        if let Ok(config) = Self::from_env() {
            info!("Loaded config from environment variables");
            return Ok(config);
        }

        // Fall back to defaults
        warn!("No config file found, using defaults");
        Ok(Self::default())
    }

    /// Load configuration from a file (YAML or TOML)
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()
            .map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Failed to load config file: {}", e))
            })?;

        let config: Config = settings.try_deserialize().map_err(|e| {
            crate::TaskQueueError::ConfigError(format!("Failed to parse config: {}", e))
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> crate::Result<Self> {
        let mut config = Self::default();
        let mut found_any = false;

        if let Ok(val) = env::var("TASK_QUEUE_WORKER_COUNT") {
            config.worker_count = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid WORKER_COUNT: {}", e))
            })?;
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_MAX_QUEUE_SIZE") {
            config.max_queue_size = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid MAX_QUEUE_SIZE: {}", e))
            })?;
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_TASK_TIMEOUT_SECS") {
            config.task_timeout_secs = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid TASK_TIMEOUT_SECS: {}", e))
            })?;
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_MAX_RETRIES") {
            config.max_retries = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid MAX_RETRIES: {}", e))
            })?;
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_SHUTDOWN_TIMEOUT_SECS") {
            config.shutdown_timeout_secs = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid SHUTDOWN_TIMEOUT_SECS: {}", e))
            })?;
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_STORAGE_BACKEND") {
            config.storage_backend = match val.to_lowercase().as_str() {
                "memory" => StorageBackend::Memory,
                "redis" => StorageBackend::Redis,
                "postgresql" | "postgres" => StorageBackend::PostgreSQL,
                _ => {
                    return Err(crate::TaskQueueError::ConfigError(format!(
                        "Invalid STORAGE_BACKEND: {}",
                        val
                    )))
                }
            };
            found_any = true;
        }

        if let Ok(val) = env::var("TASK_QUEUE_ENABLE_PERSISTENCE") {
            config.enable_persistence = val.parse().map_err(|e| {
                crate::TaskQueueError::ConfigError(format!("Invalid ENABLE_PERSISTENCE: {}", e))
            })?;
            found_any = true;
        }

        if !found_any {
            return Err(crate::TaskQueueError::ConfigError(
                "No environment variables found".to_string(),
            ));
        }

        config.validate()?;
        Ok(config)
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
