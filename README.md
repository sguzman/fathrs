# Rust Template

This repository is a minimal, opinionated Rust template for starting new projects.
It focuses on consistent tooling, formatting, and release hygiene so new crates
share the same baseline from day one.

## Goals and intent

- Provide a clean starting point for a binary crate.
- Keep the project layout small and easy to extend.
- Standardize formatting, linting, and release workflows.

## Repository structure

- `src/` - Application source code.
- `docs/` - Documentation and AI/workflow rules.
- `Cargo.toml` - Crate metadata and dependencies.
- `justfile` - Common developer tasks.
- Tooling config: `rustfmt.toml`, `taplo.toml`, `typos.toml`, `biome.json`,
  `lychee.toml`, `release.toml`, `cliff.toml`.

## Getting started

1. Update `Cargo.toml` with your crate name and metadata.
2. Replace `src/main.rs` with your application.
3. Run `cargo build` to verify the toolchain.
