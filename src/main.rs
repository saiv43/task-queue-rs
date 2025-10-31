//! Task Queue RS binary entry point

use task_queue_rs::{config::Config, queue::memory::MemoryQueue, worker::pool::WorkerPool};
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
    config.validate()?;

    info!(
        "Initialized with {} workers, max queue size: {}",
        config.worker_count, config.max_queue_size
    );

    // Create queue and worker pool
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(config.worker_count);

    // Start processing tasks
    worker_pool.start(queue).await?;

    Ok(())
}
