# Default recipe: list available commands
default:
    @just --list

# Run all workspace tests
test:
    cargo test --workspace

# Run a specific crate's tests
test-crate crate:
    cargo test -p {{crate}}

# Run tests with coverage (HTML report in target/llvm-cov/html/)
coverage:
    cargo llvm-cov --workspace --html
    @echo "Report: target/llvm-cov/html/index.html"

# Run tests with coverage (text summary)
coverage-summary:
    cargo llvm-cov --workspace

# Run tests with coverage (LCOV output for CI)
coverage-lcov:
    cargo llvm-cov --workspace --lcov --output-path target/llvm-cov/lcov.info

# Check formatting
fmt-check:
    cargo fmt --check

# Format all code
fmt:
    cargo fmt

# Run clippy lints
clippy:
    cargo clippy --workspace -- -D warnings

# Run all checks (fmt + clippy + tests)
check: fmt-check clippy test

# Build the workspace
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# Run the opt-in acpx compatibility harness against brain-acp
acpx-compat:
    ./scripts/test-acpx-compat.sh

# Install repo-owned Nori custom prompts into ~/.nori/cli/commands via symlink
nori-prompts-install:
    ./scripts/install-nori-prompts.sh

# Install the pinned Harbor CLI as an external tool
harbor-install:
    ./scripts/harbor-install.sh

# List datasets exposed by the live Harbor registry
harbor-datasets:
    harbor datasets list

# Run one Harbor agent against one Harbor dataset and optional task name.
harbor-run agent dataset task_name='':
    bash ./scripts/harbor-run.sh {{agent}} {{dataset}} '{{task_name}}'

# Browse Harbor job trajectories in the built-in web viewer
harbor-view-jobs:
    harbor view target/harbor/jobs

# Clean build artifacts
clean:
    cargo clean
