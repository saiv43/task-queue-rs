# Task Queue RS

A high-performance distributed task queue system written in Rust.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/task-queue-rs.git
cd task-queue-rs

# Run with Make
make run

# Or run with Cargo
cargo run

# Run tests
make test

# Run with Docker
make docker-build
make docker-run
```

## Features

- **Asynchronous Processing**: Built on Tokio for efficient async task execution
- **Priority Queue**: Support for task priorities (Low, Normal, High, Critical)
- **Task Scheduling**: Schedule tasks for delayed or future execution
- **In-Memory Queue**: Fast in-memory task queue (base implementation)
- **Worker Pool**: Configurable worker pool for concurrent task processing
- **Graceful Shutdown**: Signal handling (SIGTERM/SIGINT) with configurable timeout
- **Configuration Files**: Load settings from YAML/TOML files or environment variables
- **Task Persistence**: Pluggable storage backends
- **Thread-Safe**: Race condition-free concurrent operations with atomic lock management
- **Type-Safe**: Leverages Rust's type system for safe task handling
- **Extensible**: Easy to add new storage backends and task types

## Getting Started

### Prerequisites

- Rust
- Cargo
- Docker

### Installation

```bash
# Using Cargo
cargo build --release

# Or using Make
make build
```

### Running

```bash
# Using Cargo
cargo run

# Or using Make
make run

# Development mode with debug logs
make dev
```

The service will start and wait for tasks. To stop gracefully, press `CTRL+C` or send SIGTERM:

```bash
# Graceful shutdown
kill -SIGTERM <pid>
```

The service will complete in-flight tasks before shutting down (default timeout: 30 seconds).

### Running Tests

```bash
# Using Cargo
cargo test

# Or using Make
make test

# Verbose test output
make test-verbose
```

### Using Make

The project includes a Makefile for common tasks:

```bash
# Show all available commands
make help

# Development
make build          # Build the project
make run            # Run the application
make test           # Run tests
make fmt            # Format code
make lint           # Run clippy

# Production
make release        # Build optimized binary
make install        # Install binary

# Docker
make docker-build   # Build Docker image
make docker-run     # Run in Docker
```

### Docker

Build and run with Docker:

```bash
# Build Docker image
docker build -t task-queue-rs .

# Run container
docker run --rm task-queue-rs

# Or use Make
make docker-build
make docker-run

# Using Docker Compose
docker-compose up --build
```

## Architecture

The system consists of several core components:

- **Queue**: Manages task storage and retrieval
- **Worker Pool**: Executes tasks concurrently
- **Task Executor**: Handles individual task execution
- **Storage Backend**: Pluggable persistence layer

## Configuration

The task queue can be configured using:
1. Configuration files (YAML or TOML)
2. Environment variables
3. Default values

### Configuration File

Create a `config.yaml` or `config.toml` file:

```yaml
# config.yaml
worker_count: 8
max_queue_size: 50000
task_timeout_secs: 600
max_retries: 5
storage_backend: "memory"
enable_persistence: false
shutdown_timeout_secs: 60
```

```toml
# config.toml
worker_count = 8
max_queue_size = 50000
task_timeout_secs = 600
max_retries = 5
storage_backend = "memory"
enable_persistence = false
shutdown_timeout_secs = 60
```

### Environment Variables

Override configuration with environment variables:

```bash
export TASK_QUEUE_WORKER_COUNT=16
export TASK_QUEUE_MAX_QUEUE_SIZE=100000
export TASK_QUEUE_STORAGE_BACKEND=redis
cargo run
```

### Configuration Priority

1. Environment variable `TASK_QUEUE_CONFIG` (path to config file)
2. `./config.yaml` or `./config.toml`
3. `./config/config.yaml` or `./config/config.toml`
4. Environment variables (`TASK_QUEUE_*`)
5. Default values

### Available Settings

| Setting | Environment Variable | Default | Description |
|---------|---------------------|---------|-------------|
| `worker_count` | `TASK_QUEUE_WORKER_COUNT` | CPU count | Number of worker threads |
| `max_queue_size` | `TASK_QUEUE_MAX_QUEUE_SIZE` | 10000 | Maximum tasks in queue |
| `task_timeout_secs` | `TASK_QUEUE_TASK_TIMEOUT_SECS` | 300 | Task execution timeout |
| `max_retries` | `TASK_QUEUE_MAX_RETRIES` | 3 | Max retry attempts |
| `storage_backend` | `TASK_QUEUE_STORAGE_BACKEND` | memory | Storage backend type |
| `enable_persistence` | `TASK_QUEUE_ENABLE_PERSISTENCE` | false | Enable persistence |
| `shutdown_timeout_secs` | `TASK_QUEUE_SHUTDOWN_TIMEOUT_SECS` | 30 | Graceful shutdown timeout |

## Usage Example

```rust
use task_queue_rs::{Queue, Task, WorkerPool};

#[tokio::main]
async fn main() {
    let queue = Queue::new();
    let worker_pool = WorkerPool::new(4);
    
    // Enqueue a task
    let task = Task::new("example_task", serde_json::json!({"key": "value"}));
    queue.enqueue(task).await.unwrap();
    
    // Start processing
    worker_pool.start(queue).await;
}
```

### Using Task Priorities

```rust
use task_queue_rs::{Priority, Task, TaskPayload};

// Create tasks with different priorities
let background = Task::new(cleanup_payload)
    .with_priority(Priority::Low);

let user_request = Task::new(api_payload)
    .with_priority(Priority::High);

let urgent = Task::new(critical_payload)
    .with_priority(Priority::Critical);

// Enqueue in any order
queue.enqueue(background).await.unwrap();
queue.enqueue(user_request).await.unwrap();
queue.enqueue(urgent).await.unwrap();

// Tasks dequeued by priority: Critical -> High -> Low
```

### Updating Tasks in Queue

```rust
use task_queue_rs::{Queue, Task, TaskPayload};

// Enqueue a task
let mut task = Task::new(payload);
let task_id = task.id.clone();
queue.enqueue(task.clone()).await.unwrap();

// Update task properties (priority, scheduling, payload, etc.)
task.priority = Priority::High;
task.scheduled_at = Some(Utc::now() + Duration::hours(1));
queue.update(task).await.unwrap();

// The updated task will be dequeued with new properties
// Both the priority heap and task map are kept synchronized
```

**Important Notes:**
- `update()` maintains consistency between internal data structures
- Updates affect priority ordering and scheduling
- Cannot update tasks that have already been dequeued
- Updates are atomic and thread-safe

### Scheduling Tasks

Schedule tasks for delayed or future execution:

```rust
use chrono::{Duration, Utc};
use task_queue_rs::{Task, TaskPayload};

// Schedule task for specific time
let scheduled_time = Utc::now() + Duration::hours(2);
let task = Task::new(payload)
    .schedule_at(scheduled_time);

// Schedule task with delay
let task = Task::new(payload)
    .schedule_after(Duration::minutes(30));

// Immediate execution (default)
let task = Task::new(payload); // No scheduling

// Check if task is ready
if task.is_ready() {
    // Task can be executed now
}

// Get time until ready
if let Some(duration) = task.time_until_ready() {
    println!("Task ready in {} seconds", duration.num_seconds());
}
```

#### Common Scheduling Use Cases

```rust
// Retry with exponential backoff
let retry_count = 2u32;
let retry_task = failed_task
    .schedule_after(Duration::seconds(2_i64.pow(retry_count)));

// Schedule daily cleanup job
let tomorrow_midnight = Utc::now()
    .date_naive()
    .succ_opt()
    .unwrap()
    .and_hms_opt(0, 0, 0)
    .unwrap()
    .and_utc();
let cleanup = Task::new(cleanup_payload)
    .schedule_at(tomorrow_midnight);

// Rate limiting - defer task by 1 second
let rate_limited = Task::new(api_call_payload)
    .schedule_after(Duration::seconds(1));

// Reminder notification (1 hour before event)
let reminder = Task::new(notification_payload)
    .schedule_at(event_time - Duration::hours(1));

// Combine priority and scheduling
let urgent_later = Task::new(payload)
    .with_priority(Priority::High)
    .schedule_after(Duration::minutes(5));
```

#### Scheduling Behavior

- Tasks scheduled in the past execute immediately
- `dequeue()` only returns tasks that are ready (scheduled_at <= now)
- Priority ordering is maintained among ready tasks
- Workers efficiently wait when no tasks are ready (no busy-waiting)
- Backward compatible - tasks without scheduling work as before

### Priority Levels

- **Critical** - Urgent, time-sensitive operations
- **High** - User-facing requests, important operations  
- **Normal** - Default priority for most tasks
- **Low** - Background jobs, cleanup, maintenance

## Concurrency Safety

The task queue is designed to be thread-safe and race condition-free:

- **Atomic Operations**: All queue operations (enqueue/dequeue) acquire necessary locks atomically
- **Consistency Guarantees**: The internal priority heap and task map are kept synchronized
- **No Duplicate Processing**: Each task is guaranteed to be dequeued exactly once
- **High Concurrency**: Tested under high contention with multiple concurrent workers
- **Lock Ordering**: Consistent lock acquisition order prevents deadlocks

### Concurrency Testing

The codebase includes comprehensive concurrency tests:
- Concurrent dequeue operations with duplicate detection
- Task map consistency verification under load
- High contention stress tests with 20+ workers
- Mixed concurrent enqueue/dequeue operations
- Internal consistency verification

## Roadmap

### Completed ✅
- [x] **Priority queue implementation** - Support for Low, Normal, High, Critical priorities
- [x] **Graceful shutdown** - Signal handling with configurable timeout
- [x] **Configuration files** - YAML/TOML config with environment variable overrides
- [x] **Memory leak fixes** - Proper cleanup in MemoryQueue
- [x] **Race condition fixes** - Thread-safe concurrent operations
- [x] **Task scheduling** - Delayed and future task execution with `schedule_at()` and `schedule_after()`
- [x] **Queue update fix** - Proper synchronization between heap and task_map on updates

### Planned 🚧
- [ ] Redis backend support
- [ ] PostgreSQL backend support
- [ ] Recurring/periodic tasks (cron-like scheduling)
- [ ] Enhanced retry mechanism with exponential backoff
- [ ] Dead letter queue
- [ ] Metrics and monitoring (Prometheus)
- [ ] REST API
- [ ] gRPC support
- [ ] Task dependencies
- [ ] Rate limiting
- [ ] Web dashboard
- [ ] Docker support
- [ ] Clustering support

## CI/CD Pipeline

This project uses GitHub Actions for automated testing and quality checks. The workflow runs on every pull request and push to main.

### Workflow

The CI workflow runs a single job that executes `make ci`, which includes:
- ✅ Code formatting check (`cargo fmt`)
- ✅ Linting with strict rules (`cargo clippy`)
- ✅ All tests
- ✅ Compilation check (`cargo check`)

### Pull Request Requirements

All pull requests must pass the CI workflow, which enforces:
- ✅ All tests passing
- ✅ Code formatting check
- ✅ Clippy linting with strict rules (no warnings, unwraps, or panics)
- ✅ All public items documented

### Running CI Checks Locally

Before pushing, run the same checks locally:

```bash
# Run all CI checks
make ci

# Or run individually
cargo check --all-targets
cargo test --verbose
cargo fmt -- --check
cargo clippy -- -D warnings -D clippy::unwrap_used -D clippy::panic -D missing_docs
```

## Development

### Code Quality Standards

- **No panics**: Use `Result` types and proper error handling
- **No unwraps**: Handle errors explicitly
- **Documentation**: All public APIs must be documented
- **Testing**: All features must have tests
- **Formatting**: Code must be formatted with `rustfmt`

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test shutdown_test

# Run with output
cargo test -- --nocapture

# Run tests in watch mode (requires cargo-watch)
cargo watch -x test
```
