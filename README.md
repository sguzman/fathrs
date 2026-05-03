# fathrs

`fathrs` is a minimal Rust dotfile linker that reads a TOML spec and creates symlinks from target paths to source paths.

## Intent

Provide a small, explicit, and observable alternative to heavier dotfile managers by doing one thing well: linking files.

## Ambition

The current docs make the ambition explicit by omission: keep the tool narrow, predictable, and intentionally free of templating or rendering layers.

## Current Status

The CLI, schema, examples, tests, and release-oriented housekeeping are already present. The repository looks disciplined and close to a stable single-purpose utility.

## Core Capabilities Or Focus Areas

- Read link definitions from TOML.
- Create symlinks for dotfile deployment.
- Support dry-run and replace-style workflows.
- Emit status/reporting output for applied links.
- Keep behavior intentionally constrained to linking.

## Project Layout

- `docs/`: project documentation, reference material, and roadmap notes.
- `examples/`: sample inputs, example configs, or demonstration workflows.
- `schema/`: schema files or schema-oriented reference material.
- `src/`: Rust source for the main crate or application entrypoint.
- `tests/`: automated tests, fixtures, or parity scenarios.
- `Cargo.toml`: crate or workspace manifest and the first place to check for package structure.

## Setup And Requirements

- Rust toolchain.
- A `links.toml`-style config file.
- A platform/filesystem setup where symlinks are supported and expected.

## Build / Run / Test Commands

```bash
cargo build
cargo test
cargo run -- --help
```

## Notes, Limitations, Or Known Gaps

- The narrow scope is deliberate; template rendering and broader config-management behaviors are out of scope.
- Filesystem semantics differ across platforms, so test your link targets before widespread use.

## Next Steps Or Roadmap Hints

- Preserve the project's simplicity as features are proposed.
- Document platform-specific symlink caveats as they arise.
