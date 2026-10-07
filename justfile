set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @just --list

build:
  cargo build --all-targets --all-features

check:
  cargo check --all-targets --all-features

fmt:
  cargo fmt

fmt-check:
  cargo fmt --check

clippy:
  cargo clippy --all-targets --all-features -- -D warnings

test:
  cargo test --all-features

doc:
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

ci: fmt-check check clippy test doc

install:
  cargo install --path .
