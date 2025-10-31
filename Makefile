help:
	@echo "Task Queue RS - Available Commands"
	@echo "===================================="
	@echo "Development:"
	@echo "  make build        - Build the project in debug mode"
	@echo "  make run          - Run the application"
	@echo "  make dev          - Run in development mode with logs"
	@echo "  make test         - Run all tests"
	@echo "  make check        - Run cargo check"
	@echo "  make fmt          - Format code with rustfmt"
	@echo "  make lint         - Run clippy linter"
	@echo ""
	@echo "Production:"
	@echo "  make release      - Build optimized release binary"
	@echo "  make install      - Install the binary to cargo bin"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build - Build Docker image"
	@echo "  make docker-run   - Run application in Docker"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean        - Remove build artifacts"

# Run

run:
	@echo "Running Task Queue RS..."
	cargo run

docker-run:
	@echo "Running Docker container..."
	docker run --rm task-queue-rs:latest

docker-compose-up:
	@echo "Starting services with Docker Compose..."
	docker-compose up --build

fmt:
	@echo "Formatting code..."
	rustup component add rustfmt
	cargo fmt

build:
	@echo "Building project..."
	cargo build

release:
	@echo "Building release version..."
	cargo build --release

# CI

ci: fmt-check lint test check
	@echo "CI checks passed!"

fmt-check:
	@echo "Checking code formatting..."
	rustup component add rustfmt
	cargo fmt -- --check

lint:
	@echo "Running clippy..."
	rustup component add clippy
	cargo clippy -- \
		-D warnings \
		-D clippy::unwrap_used \
		-D clippy::panic \
		-D missing_docs

test:
	@echo "Running tests..."
	cargo test

check:
	@echo "Checking code..."
	cargo check

# Misc

clean:
	@echo "Cleaning build artifacts..."
	cargo clean

bench:
	@echo "Running benchmarks..."
	cargo bench

docs:
	@echo "Generating documentation..."
	cargo doc --no-deps --open

.PHONY: help run docker-run docker-compose-up,  fmt, build, release, ci, fmt-check, lint, test, check, clean, bench, docs