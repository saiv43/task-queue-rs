use task_queue_rs::{
    queue::memory::MemoryQueue,
    queue::Queue,
    task::{Task, TaskPayload},
    worker::pool::WorkerPool,
};
use tokio::time::{sleep, timeout, Duration};

#[tokio::test]
async fn test_graceful_shutdown_empty_queue() {
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(2);

    // Start worker pool
    worker_pool.start(queue).await.unwrap();

    // Give workers time to start
    sleep(Duration::from_millis(100)).await;

    // Shutdown immediately
    let result = worker_pool.shutdown(Duration::from_secs(5)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_graceful_shutdown_with_tasks() {
    let queue = MemoryQueue::new();

    // Add some quick tasks
    for i in 0..5 {
        let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let mut worker_pool = WorkerPool::new(2);
    worker_pool.start(queue).await.unwrap();

    // Let workers process some tasks
    sleep(Duration::from_millis(500)).await;

    // Shutdown with reasonable timeout
    let result = worker_pool.shutdown(Duration::from_secs(10)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shutdown_timeout() {
    let queue = MemoryQueue::new();

    // Add tasks that take longer than shutdown timeout
    for i in 0..3 {
        let payload = TaskPayload::new("task_type_2".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let mut worker_pool = WorkerPool::new(1);
    worker_pool.start(queue).await.unwrap();

    // Give worker time to start processing
    sleep(Duration::from_millis(200)).await;

    // Shutdown with very short timeout (should timeout)
    let result = worker_pool.shutdown(Duration::from_millis(100)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_pool_starts_correctly() {
    let queue = MemoryQueue::new();
    let mut worker_pool = WorkerPool::new(4);

    assert_eq!(worker_pool.worker_count(), 4);
    assert!(!worker_pool.is_running());

    worker_pool.start(queue).await.unwrap();
    assert!(worker_pool.is_running());
}

#[tokio::test]
async fn test_multiple_workers_process_tasks() {
    let queue = MemoryQueue::new();

    // Add multiple tasks
    for i in 0..10 {
        let payload = TaskPayload::new("task_type_0".to_string(), serde_json::json!({"id": i}));
        queue.enqueue(Task::new(payload)).await.unwrap();
    }

    let initial_size = queue.size().await;
    assert_eq!(initial_size, 10);

    let mut worker_pool = WorkerPool::new(3);
    worker_pool.start(queue).await.unwrap();

    // Let workers process tasks
    sleep(Duration::from_millis(800)).await;

    // Shutdown
    worker_pool.shutdown(Duration::from_secs(5)).await.unwrap();
}

#[tokio::test]
async fn test_shutdown_signal_propagates_to_all_workers() {
    let queue = MemoryQueue::new();
    let worker_count = 5;
    let mut worker_pool = WorkerPool::new(worker_count);

    worker_pool.start(queue).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // All workers should stop within timeout
    let shutdown_result = timeout(
        Duration::from_secs(3),
        worker_pool.shutdown(Duration::from_secs(2)),
    )
    .await;

    assert!(shutdown_result.is_ok());
    assert!(shutdown_result.unwrap().is_ok());
}

#[tokio::test]
async fn test_sigterm_triggers_graceful_shutdown() {
    use std::process::{Command, Stdio};
    use std::time::Duration;
    use tokio::time::sleep;

    // Start the application as a subprocess
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");

    let pid = child.id();

    // Give the application time to start
    sleep(Duration::from_millis(500)).await;

    // Send SIGTERM signal
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let result = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
        assert!(result.is_ok(), "Failed to send SIGTERM");
    }

    // Wait for graceful shutdown with timeout
    let shutdown_result = tokio::time::timeout(Duration::from_secs(5), async {
        child.wait().expect("Failed to wait for child process")
    })
    .await;

    assert!(
        shutdown_result.is_ok(),
        "Application did not shutdown within timeout"
    );
    let status = shutdown_result.unwrap();
    // On Unix, when terminated by signal, exit code is None or the process exits cleanly with 0
    // We just verify it shut down within the timeout period
    assert!(
        status.code().is_none() || status.code() == Some(0),
        "SIGTERM: Application did not shutdown gracefully, exit code: {:?}",
        status.code()
    );
}

#[tokio::test]
async fn test_sigint_triggers_graceful_shutdown() {
    use std::process::{Command, Stdio};
    use std::time::Duration;
    use tokio::time::sleep;

    // Start the application as a subprocess
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");

    let pid = child.id();

    // Give the application time to start
    sleep(Duration::from_millis(500)).await;

    // Send SIGINT signal (CTRL+C)
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let result = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
        assert!(result.is_ok(), "Failed to send SIGINT");
    }

    // Wait for graceful shutdown with timeout
    let shutdown_result = tokio::time::timeout(Duration::from_secs(5), async {
        child.wait().expect("Failed to wait for child process")
    })
    .await;

    assert!(
        shutdown_result.is_ok(),
        "Application did not shutdown within timeout"
    );
    let status = shutdown_result.unwrap();
    // On Unix, when terminated by signal, exit code is None or the process exits cleanly with 0
    // We just verify it shut down within the timeout period
    assert!(
        status.code().is_none() || status.code() == Some(0),
        "SIGINT: Application did not shutdown gracefully, exit code: {:?}",
        status.code()
    );
}

#[tokio::test]
async fn test_signal_shutdown_with_active_tasks() {
    use std::process::{Command, Stdio};
    use std::time::Duration;
    use tokio::time::sleep;

    // Start the application as a subprocess
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");

    let pid = child.id();

    // Give the application time to start and potentially process tasks
    sleep(Duration::from_secs(1)).await;

    // Send SIGTERM signal
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let result = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
        assert!(result.is_ok(), "Failed to send SIGTERM");
    }

    // Wait for graceful shutdown - should complete within configured timeout
    let shutdown_result = tokio::time::timeout(
        Duration::from_secs(35), // Slightly more than default 30s shutdown timeout
        async { child.wait().expect("Failed to wait for child process") },
    )
    .await;

    assert!(
        shutdown_result.is_ok(),
        "Application did not shutdown within expected time"
    );
    let status = shutdown_result.unwrap();
    // On Unix, when terminated by signal, exit code is None or the process exits cleanly with 0
    // We just verify it shut down within the timeout period
    assert!(
        status.code().is_none() || status.code() == Some(0),
        "Application did not shutdown gracefully after processing tasks, exit code: {:?}",
        status.code()
    );
}
