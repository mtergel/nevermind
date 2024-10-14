ifneq (,$(wildcard ./.env))
    include .env
    export
endif

# Setup
.PHONY: setup
setup:
	sqlx db setup

# Build the project
.PHONY: build
build:
	cargo build

# Run the project, Dev loop
.PHONY: run
run: build
	cargo watch -x run | bunyan

# Test the project
.PHONY: test
test:
	RUST_LOG=nevermind=trace,tower_http=debug,axum::rejection=trace cargo test

# Run migrations
.PHONY: migrate
migrate:
	sqlx migrate run

# Help
.PHONY: help
help:
	@echo "Makefile commands:"
	@echo "  make setup   - Setup the project"
	@echo "  make build   - Build the project"
	@echo "  make run     - Run the project"
	@echo "  make migrate - Run migrations"
	@echo "  make test    - Run tests"
	@echo "  make help    - Show this help message"
