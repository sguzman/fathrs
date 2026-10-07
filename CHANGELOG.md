# Changelog

## [1.4.0] - 2026-10-07

This release re-establishes Fathrs as a small, production-usable dotfile
deployer after the repository accumulated stale template infrastructure and
untested filesystem behavior.

### Features

- Build and validate a complete deployment plan before the first filesystem
  mutation.
- Add safe `unlink` support with dry-run; only destinations still matching the
  configured state are removed.
- Make `probe` detect missing destinations, wrong filesystem kinds, wrong
  symlink targets, and drifted copied content, with non-zero exit status on
  drift.
- Add idempotent recursive copy mode for files, directories, and Unix symlink
  entries.
- Standardize privileged operations on `doas`.
- Adopt `doas = true` as the canonical configuration key while retaining
  `sudo = true` as a backward-compatible alias.

### Safety and correctness

- Fix absolute and relative `--base-dir` resolution.
- Reject duplicate resolved destinations, source/destination identity or
  containment, and dangerous direct root/home destinations.
- Ensure a missing later source cannot leave earlier entries partially applied.
- Require explicit `--force` before replacing conflicting files, directories,
  or symlinks.
- Make dry-run use the same validated plan and destination-state checks as real
  execution.
- Verify every applied change and unlink operation afterward.
- Fix lexical path normalization for repeated leading parent components.
- Route structured logs to stderr so command data on stdout remains usable.

### Testing and tooling

- Replace the old repository-mutating example test with an isolated temporary
  filesystem integration suite covering links, directories, copy drift,
  force behavior, dry runs, path resolution, probe, validation, and unlink.
- Make privileged command construction testable without invoking `doas`.
- Standardize development and CI on stable Rust.
- Add push/pull-request CI for check, strict Clippy, tests, rustdoc, install
  smoke testing, and rustfmt.
- Modernize the GitHub release workflow and release artifact packaging.
- Add a realistic EndeavourOS/Hyprland-style dotfiles example.

### Documentation

- Replace unrelated Rust-template migration/branding/AI documentation with a
  focused README, architecture notes, release policy, and recovery roadmap.
- Repair project-name and repository-link residue.

> **SemVer adoption:** v1.4.0 is the retrofit SemVer baseline. Existing tags
> v0.1.1 through v1.3.4 are preserved as legacy pre-discipline provenance and
> are not retroactively certified under the current release doctrine. The
> v1.4.0 minor classification is justified by backwards-compatible new
> capability plus fixes; adopting discipline itself does not cause a version
> bump.
