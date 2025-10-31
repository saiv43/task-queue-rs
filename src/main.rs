//! Task Queue RS binary entry point

use task_queue_rs::{config::Config, queue::memory::MemoryQueue, worker::pool::WorkerPool};
use tokio::signal;
use tokio::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Starting Task Queue RS");

    // Load configuration
    let config = Config::default();
    config
        .validate()
        .map_err(|e| format!("Configuration error: {}", e))?;

    info!(
        "Initialized with {} workers, max queue size: {}, shutdown timeout: {}s",
        config.worker_count, config.max_queue_size, config.shutdown_timeout_secs
    );

    // Create queue and worker pool
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(config.worker_count);

    // Start processing tasks
    worker_pool
        .start(queue)
        .await
        .map_err(|e| format!("Failed to start worker pool: {}", e))?;

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutdown signal received (SIGINT/SIGTERM)");

    // Perform graceful shutdown
    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);
    worker_pool
        .shutdown(shutdown_timeout)
        .await
        .map_err(|e| format!("Shutdown error: {}", e))?;

    info!("Task Queue RS shutdown complete");
    Ok(())
}
