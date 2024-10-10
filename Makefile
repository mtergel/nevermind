CARGO = cargo

.PHONY: all
all: build


# Build the project
.PHONY: build
build:
	$(CARGO) build

# Run the project
.PHONY: run
run: build
	cargo watch -x check -x test -x run

# Help
.PHONY: help
help:
	@echo "Makefile commands:"
	@echo "  make build   - Build the project"
	@echo "  make run     - Run the project"
	@echo "  make help    - Show this help message"
