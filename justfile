# List the available commands
help:
    @just --list --justfile {{justfile()}}

# Prepare the environment for development, installing all the dependencies and
# setting up the pre-commit hooks.
setup:
    uvx pre-commit install -t pre-commit

# Run the pre-commit checks
check:
    uvx pre-commit run --all-files

# Run all the rust tests
test:
    cargo test --all-features

# Auto-fix all clippy warnings
fix:
    cargo clippy --all-targets --all-features --workspace --fix --allow-staged --allow-dirty

# Format the code
format:
    cargo fmt --all

# Generate a test coverage report
coverage:
    cargo llvm-cov --lcov > lcov.info
