set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @just --list

build:
  cargo build --all-targets --all-features --locked

check:
  cargo check --all-targets --all-features --locked

fmt:
  cargo fmt

fmt-check:
  cargo fmt --check

clippy:
  cargo clippy --all-targets --all-features --locked -- -D warnings

test:
  cargo test --all-features --locked

doc:
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked

ci: check clippy test doc fmt-check

install:
  cargo install --path . --locked
