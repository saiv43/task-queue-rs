use crate::queue::Queue;
use crate::worker::Worker;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

/// A pool of workers that process tasks concurrently
pub struct WorkerPool {
    worker_count: usize,
    handles: Vec<JoinHandle<()>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl WorkerPool {
    /// Create a new worker pool with the specified number of workers
    pub fn new(worker_count: usize) -> Self {
        Self {
            worker_count,
            handles: Vec::new(),
            shutdown_tx: None,
        }
    }

    /// Start the worker pool
    pub async fn start<Q: Queue + 'static>(&mut self, queue: Q) -> crate::Result<()> {
        let queue = Arc::new(queue);
        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        info!("Starting worker pool with {} workers", self.worker_count);

        for i in 0..self.worker_count {
            let worker = Worker::new(i);
            let queue_clone = Arc::clone(&queue);
            let shutdown_rx = shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                worker.run_with_shutdown(queue_clone, shutdown_rx).await;
            });

            self.handles.push(handle);
        }

        Ok(())
    }

    /// Initiate graceful shutdown of the worker pool
    pub async fn shutdown(&mut self, timeout_duration: Duration) -> crate::Result<()> {
        info!("Initiating graceful shutdown...");

        // Send shutdown signal to all workers
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }

        info!(
            "Waiting for {} workers to complete (timeout: {}s)...",
            self.handles.len(),
            timeout_duration.as_secs()
        );

        // Wait for all workers with timeout
        let shutdown_result = timeout(timeout_duration, async {
            for (idx, handle) in self.handles.drain(..).enumerate() {
                match handle.await {
                    Ok(()) => info!("Worker {} stopped gracefully", idx),
                    Err(e) => warn!("Worker {} panicked: {}", idx, e),
                }
            }
        })
        .await;

        match shutdown_result {
            Ok(()) => {
                info!("All workers stopped successfully");
                Ok(())
            }
            Err(_) => {
                warn!("Shutdown timeout exceeded, some workers may still be running");
                Err(crate::TaskQueueError::WorkerPoolError(
                    "Shutdown timeout exceeded".to_string(),
                ))
            }
        }
    }

    /// Get the number of workers in the pool
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Check if the pool is running
    pub fn is_running(&self) -> bool {
        !self.handles.is_empty()
    }
}
