# Release policy

Fathrs uses Conventional Commit-style messages and semantic versions.

A release is allowed only when the current `main` commit passes `just ci`.

## Versioning

- Patch: fixes and internal hardening with no intended config/CLI expansion.
- Minor: backward-compatible user-facing commands, flags, or config features.
- Major: incompatible CLI/config behavior.

The repository is already beyond the original experimental API, so future
releases should describe actual shipped behavior rather than preserve stale
historical version claims.

## Release checklist

1. Ensure `main` is green in GitHub Actions.
2. Run `just ci` locally when possible.
3. Update `CHANGELOG.md`.
4. Bump `Cargo.toml`.
5. Commit the release preparation.
6. Tag the exact commit as `vX.Y.Z`.
7. Create the GitHub release for that tag.
8. Confirm the release workflow attaches the Linux archive.

Release artifacts must be built from the same tagged source that passed CI.
