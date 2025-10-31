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
- **In-Memory Queue**: Fast in-memory task queue (base implementation)
- **Worker Pool**: Configurable worker pool for concurrent task processing
- **Graceful Shutdown**: Signal handling (SIGTERM/SIGINT) with configurable timeout
- **Task Persistence**: Pluggable storage backends
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
  

## Roadmap

- [ ] Redis backend support
- [ ] PostgreSQL backend support
- [ ] Priority queue implementation
- [ ] Scheduled/delayed tasks
- [ ] Task retry mechanism
- [ ] Dead letter queue
- [ ] Metrics and monitoring
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
