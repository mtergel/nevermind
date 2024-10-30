# This Makefile is here for beginners to help manage common development tasks.
# It uses various tools like `sqlx` and `cargo` to streamline project setup, building, and testing.
# Each command is defined as a target that can be run using `make <command>`.

# If a .env file exists, load environment variables from it.
ifneq (,$(wildcard ./.env))
    include .env
    export
endif

# Setup the database and run initial migrations.
# This should be run once when starting a new project.
.PHONY: setup
setup:
	sqlx db setup

# Prepare for offline builds, including SQLx compilation checks for all targets.
# This generates a `sqlx-data.json` file that caches database queries, speeding up future builds.
#
# The offline queries must be added to version control
.PHONY: prepare
prepare:
	cargo sqlx prepare -- --all-targets

# Build the Rust project, compiling all dependencies and the project itself.
.PHONY: build
build:
	cargo build

# Run the project in development mode.
# It watches for changes in the code and automatically restarts the server when a change is detected.
# Output is piped through `bunyan` for better formatted logs.
.PHONY: dev
dev: build
	RUST_LOG=debug,tower_http=debug,axum::rejection=trace cargo watch -x run | bunyan

# Run code linting using `cargo clippy`.
# It checks for common mistakes and code style issues.
# The `-D warnings` flag treats all warnings as errors, ensuring code quality.
.PHONY: lint
lint:
	cargo clippy -- -D warnings

# Format the code using `cargo fmt` to ensure consistency.
.PHONY: fmt
format:
	cargo fmt

# Run tests using `cargo test`.
# To see test logs during test execution, run: `TEST_LOG=true make test | bunyan`.
.PHONY: test
test:
	cargo test

# Apply database migrations using `sqlx`.
# It runs any pending migrations to keep the database schema up to date.
# After running migrations, it prepares the project for offline builds.
.PHONY: migrate
migrate:
	sqlx migrate run
	$(MAKE) prepare

# Show help information for available commands in the Makefile.
# Use `make help` to see descriptions of each command.
.PHONY: help
help:
	@echo "Makefile commands:"
	@echo "  make setup   - Setup the project (initialize the database)"
	@echo "  make build   - Build the project"
	@echo "  make dev     - Run the project with auto-reloading"
	@echo "  make migrate - Apply database migrations and prepare for offline builds"
	@echo "  make test    - Run tests"
	@echo "  make lint    - Lint the codebase"
	@echo "  make format  - Format the codebase"
	@echo "  make help    - Show this help message"
