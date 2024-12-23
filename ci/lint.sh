#!/usr/bin/env bash

# Ensure the code is formatted properly
cargo fmt --all -- --check

# Check clippy lints
LINTS="
  -D warnings
  -D clippy::pedantic
"
cargo clippy --all-features --all-targets -- -D warnings -D clippy::pedantic || exit 1

# Build all targets
cargo build --verbose --all-targets
