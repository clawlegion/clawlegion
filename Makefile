# ClawLegion Makefile
# Provides convenient targets for building and managing the project

.PHONY: all build check test clean plugins clean-plugins help

# Default target
all: check build

# Full build
build:
	cargo build

# Check code without building
check:
	cargo check

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/plugins

# Build all dynamic plugins
plugins:
	@./scripts/build-plugins.sh

# Build a specific plugin by name
# Usage: make plugin NAME=openai-provider
plugin:
	@if [ -z "$(NAME)" ]; then \
		echo "Error: NAME is required. Usage: make plugin NAME=<plugin-name>"; \
		exit 1; \
	fi
	@cd plugins/$(NAME) && cargo build --release

# Clean plugin build artifacts
clean-plugins:
	rm -rf target/plugins
	@for dir in plugins/*/; do \
		rm -rf "$$dir/target"; \
	done

# Format code
fmt:
	cargo fmt

# Run clippy
lint:
	cargo clippy -- -D warnings

# Full QA workflow
qa: fmt lint check test

# Help target
help:
	@echo "ClawLegion Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all          - Check and build (default)"
	@echo "  build        - Build the project"
	@echo "  check        - Check code without building"
	@echo "  test         - Run tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  plugins      - Build all dynamic plugins"
	@echo "  plugin       - Build a specific plugin (usage: make plugin NAME=<name>)"
	@echo "  clean-plugins- Clean plugin build artifacts"
	@echo "  fmt          - Format code"
	@echo "  lint         - Run clippy"
	@echo "  qa           - Full QA: fmt, lint, check, test"
	@echo "  help         - Show this help"
