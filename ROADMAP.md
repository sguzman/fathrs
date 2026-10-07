# Fathrs Roadmap

Fathrs is a small, dependable dotfile deployer: a declarative `links.toml`
describes source-to-destination mappings, and Fathrs makes the machine match
that description with symlinks or explicit copies.

The recovery/hardening roadmap was executed on 2026-10-07. The codebase is now
ready for real dotfile use; the only remaining publication step is creating the
GitHub `v1.4.0` tag/release.

## Design Contract

Fathrs remains:

- Rust-native and Unix/Linux-focused.
- Declarative: plain TOML in, filesystem operations out.
- Small: no templating language, plugin system, profile engine, secret manager,
  package manager, or repository mutation framework.
- Predictable: dry-run and real execution consume the same validated plan.
- Idempotent: rerunning a correct configuration is a no-op.
- Conservative with destructive operations.
- Easy to install and verify.

## Phase 0 - Trustworthy baseline

- [x] Add CI for pushes and pull requests.
- [x] Standardize repository and CI on stable Rust.
- [x] Reduce verification to tools that actually belong to Fathrs.
- [x] Remove unrelated template tasks from the `justfile`.
- [x] Make `just ci` the authoritative locked local verification contract.
- [x] Run the application through check, Clippy, tests, rustdoc, install smoke
      tests, and rustfmt in CI.
- [x] Replace generic migration/template documentation with Fathrs-specific
      documentation.

Acceptance criterion met: a clean checkout has one obvious verification path,
and the full Linux CI contract passes.

## Phase 1 - Correctness bugs

- [x] Fix absolute `--base-dir` handling.
- [x] Fix relative `--base-dir` resolution against the config directory.
- [x] Use `doas` consistently for privileged copy/link/remove/mkdir operations.
- [x] Replace stale `dotlink` CLI and tracing identity with `fathrs`.
- [x] Replace obsolete integration-test CLI assumptions.
- [x] Fix lexical normalization of repeated leading `..` components.
- [x] Audit relative and broken symlink comparison.
- [x] Audit copy semantics for files, directories, and Unix symlink entries.
- [x] Align schema, parser behavior, examples, and README.

Acceptance criterion met: documented CLI/config examples match implemented
behavior.

## Phase 2 - Destructive-operation safety

- [x] Build and validate the complete plan before the first mutation.
- [x] Reject duplicate destinations, identical paths, path containment, and
      dangerous direct root/home destinations.
- [x] Ensure a missing later source prevents all earlier mutations.
- [x] Define conflict/replacement behavior for files, directories, and symlinks.
- [x] Make `--dry-run` use the exact real execution plan.
- [x] Make `--force` explicitly report replacement target/state.
- [x] Add regression coverage for recursive directory replacement and other
      destructive cases.
- [x] Decide on backup support: not added. Preflight, dry-run, explicit
      `--force`, post-change verification, and safe `unlink` keep the core
      simpler and deterministic.

Acceptance criterion met: predictable validation errors are discovered before
any configured mutation begins.

## Phase 3 - Test matrix

- [x] Test home, relative, absolute, `.`, and `..` path behavior.
- [x] Test file and directory symlinks.
- [x] Test idempotent reruns.
- [x] Test conflicts with and without `--force`.
- [x] Test wrong-target and broken symlinks.
- [x] Test recursive file/directory copy behavior and copied symlink entries.
- [x] Test section defaults and per-entry overrides for copy and privilege
      escalation.
- [x] Test `probe` and `--warn-only`.
- [x] Test dry-run filesystem immutability.
- [x] Test semantic/config validation failures.
- [x] Isolate and test `doas` command construction without elevation.
- [x] Use temporary directories instead of mutating checked-in fixtures.

Current suite: 8 unit tests plus 25 isolated CLI/filesystem integration tests.

Acceptance criterion met: ordinary filesystem behavior is comprehensively
testable without root or doas.

## Phase 4 - Configuration model

- [x] Make `doas = true` the canonical privilege flag.
- [x] Preserve `sudo = true` as a backward-compatible alias.
- [x] Tighten the JSON schema for detailed entries and conflicting privilege
      keys.
- [x] Reject unknown detailed-entry fields in the actual Serde parser too.
- [x] Make `validate` perform semantic plan validation without filesystem
      mutation or requiring source files to exist.
- [x] Include section/path context in operational validation diagnostics.
- [x] Document and test path-resolution rules.

Acceptance criterion met: configuration validity can be checked without touching
the filesystem.

## Phase 5 - Probe/status

- [x] Compare resolved symlink targets with configured sources.
- [x] Distinguish missing, correct, wrong-kind, wrong-target, and drifted copy
      states.
- [x] Return non-zero when drift is detected.
- [x] Keep `--warn-only` quiet for healthy entries.
- [x] Separate configured `doas` preference from an OS-level write/search
      access check.
- [x] Keep structured diagnostics on stderr and stable probe data on stdout.

Acceptance criterion met: `fathrs probe` is a reliable machine-state check.

## Phase 6 - Installation and release hygiene

- [x] Synchronize crate version and changelog for `1.4.0`.
- [x] Repair stale `sguzman/fathers` links and old project-name residue.
- [x] Replace deprecated/broken release workflow pieces.
- [x] Make release artifacts run the same locked verification gates as CI.
- [x] Standardize on stable Rust.
- [x] Verify `cargo install --path . --locked`.
- [x] Verify `cargo install --git https://github.com/sguzman/fathrs --rev <SHA>
      --locked` in CI.
- [ ] Publish the real GitHub `v1.4.0` tag/release and confirm the release
      workflow attaches the Linux archive.

Engineering/release preparation is complete. The unchecked item is the external
GitHub publication action; the available repository automation used for this
recovery does not expose tag/release creation.

## Phase 7 - Daily-use ergonomics

- [x] Add a realistic EndeavourOS/Hyprland-style dotfiles fixture without
      private machine data.
- [x] Demonstrate one config mixing linked user config, copied files, and
      privileged system destinations.
- [x] Add safe `unlink` with the same plan-first model and dry-run support.
      Unlink refuses drifted/unexpected destinations.
- [x] Consider machine-readable probe output: deferred because there is no
      consumer requiring it; stdout is already stable and concise.
- [x] Improve output separation/readability without hiding filesystem actions.

Acceptance criterion met: real Linux dotfiles can be described without adding
framework machinery.

## Phase 8 - Documentation cleanup

- [x] Keep the README focused on purpose, install, config, commands, examples,
      and safety behavior.
- [x] Remove unrelated template/reference and obsolete fixture material.
- [x] Keep a short architecture document covering execution invariants.
- [x] Keep release/changelog documentation coherent with the repaired project.
- [x] Preserve a realistic example instead of repository-mutating test fixtures.

Acceptance criterion met: the README is sufficient to install, configure, probe,
deploy, and safely remove managed entries.

## Verification Record

The repaired `1.4.0` source has passed the complete CI contract on Linux:

- Cargo check with `--locked`
- Clippy across all targets/features with warnings denied
- 33 tests (8 unit + 25 integration)
- rustdoc with warnings denied
- local-path installation smoke test
- GitHub-source installation smoke test at the exact commit SHA
- rustfmt check

No major feature expansion is planned before real-world dogfooding identifies a
specific need.
