//! Executor

use crate::task::Task;
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Trait for executing tasks
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task and return the result
    async fn execute(&self, task: &mut Task) -> crate::Result<()>;
}

/// Default task executor implementation
pub struct DefaultExecutor {
    timeout_duration: Duration,
}

impl DefaultExecutor {
    /// Create a new default executor
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout_duration: Duration::from_secs(timeout_secs),
        }
    }

    /// Process the task based on its type
    async fn process_task(&self, task: &Task) -> crate::Result<()> {
        info!(
            "Processing task {} of type {}",
            task.id, task.payload.task_type
        );

        // Simulate different task types
        match task.payload.task_type.as_str() {
            "task_type_0" => self.handle_type_0(task).await,
            "task_type_1" => self.handle_type_1(task).await,
            "task_type_2" => self.handle_type_2(task).await,
            _ => self.handle_default(task).await,
        }
    }

    async fn handle_type_0(&self, task: &Task) -> crate::Result<()> {
        // Simulate quick task
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("Task {} (type 0) completed quickly", task.id);
        Ok(())
    }

    async fn handle_type_1(&self, task: &Task) -> crate::Result<()> {
        // Simulate medium task
        tokio::time::sleep(Duration::from_millis(500)).await;
        info!("Task {} (type 1) completed", task.id);
        Ok(())
    }

    async fn handle_type_2(&self, task: &Task) -> crate::Result<()> {
        // Simulate longer task
        tokio::time::sleep(Duration::from_secs(1)).await;
        info!(
            "Task {} (type 2) completed after longer processing",
            task.id
        );
        Ok(())
    }

    async fn handle_default(&self, task: &Task) -> crate::Result<()> {
        // Default handler
        tokio::time::sleep(Duration::from_millis(200)).await;
        info!("Task {} (default type) completed", task.id);
        Ok(())
    }
}

impl Default for DefaultExecutor {
    fn default() -> Self {
        Self::new(300)
    }
}

#[async_trait]
impl TaskExecutor for DefaultExecutor {
    async fn execute(&self, task: &mut Task) -> crate::Result<()> {
        task.mark_running();

        let result = timeout(self.timeout_duration, self.process_task(task)).await;

        match result {
            Ok(Ok(())) => {
                task.mark_completed();
                info!("Task {} completed successfully", task.id);
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("Task execution error: {e}");
                error!("{error_msg}");
                task.mark_failed(error_msg.clone());
                Err(crate::TaskQueueError::ExecutionFailed(error_msg))
            }
            Err(_) => {
                let error_msg = format!("Task {} timed out", task.id);
                warn!("{error_msg}");
                task.mark_failed(error_msg.clone());
                Err(crate::TaskQueueError::ExecutionFailed(error_msg))
            }
        }
    }
}
