set shell := ["fish", "-c"]

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

# One-time retrofit ceremony. Once v1.4.0 exists, never run this again.
semver-baseline:
  @test (git branch --show-current) = main; or begin; echo "SemVer baseline must be created from main."; exit 1; end
  @git diff --quiet; and git diff --cached --quiet; or begin; echo "Working tree must be clean."; exit 1; end
  just ci
  @if git rev-parse -q --verify refs/tags/v1.4.0 >/dev/null
      set tagged (git rev-parse 'v1.4.0^{}')
      set head (git rev-parse HEAD)
      test "$tagged" = "$head"; or begin; echo "v1.4.0 already exists at a different commit."; exit 1; end
      echo "v1.4.0 already exists at this commit; pushing existing baseline."
    else
      git tag -a v1.4.0 -m "fathrs 1.4.0 - SemVer adoption baseline"
    end
  git push origin main
  git push origin v1.4.0

release-baseline-check:
  @git rev-parse -q --verify refs/tags/v1.4.0 >/dev/null; or begin; echo "Missing SemVer baseline tag v1.4.0. Run: just semver-baseline"; exit 1; end

release-dry-run level: release-baseline-check
  just ci
  cargo release {{level}}

release level: release-baseline-check
  just ci
  cargo release {{level}} --execute

release-artifact:
  cargo build --release --locked
  @set version (grep '^version = ' Cargo.toml | head -n1 | cut -d '"' -f2); mkdir -p dist; tar -C target/release -cJf "dist/fathrs-v$version-x86_64-linux.tar.xz" fathrs; echo "dist/fathrs-v$version-x86_64-linux.tar.xz"
