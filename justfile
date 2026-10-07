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

release-baseline-check:
  @git rev-parse -q --verify refs/tags/v1.4.0 >/dev/null || { echo "Missing SemVer baseline tag v1.4.0. Complete docs/RELEASE.md baseline bootstrap first."; exit 1; }

release-dry-run level: release-baseline-check
  just ci
  cargo release {{level}}

release level: release-baseline-check
  just ci
  cargo release {{level}} --execute

release-artifact:
  cargo build --release --locked
  @version="$(awk -F '\"' '/^version = / { print $2; exit }' Cargo.toml)"; mkdir -p dist; tar -C target/release -cJf "dist/fathrs-v$version-x86_64-linux.tar.xz" fathrs; echo "dist/fathrs-v$version-x86_64-linux.tar.xz"
