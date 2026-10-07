# Versioning state

Fathrs uses **retrofit Semantic Versioning**.

Rigorous SemVer governance was introduced after the project already had a real
version/tag history. That history is preserved as provenance rather than
rewritten to pretend the current discipline existed from the beginning.

## Lifecycle

- regime: `maintained`
- current version: `1.4.0`
- historical MVP/version milestone: `v1.0.0` exists, but its original release
  process predates current doctrine
- SemVer adoption mode: `retrofit`
- SemVer adoption boundary commit: `f1654719399c9c0536df06e00bffb13b20876aa8`
- SemVer adoption baseline tag: `v1.4.0`
- latest pre-adoption release/tag: `v1.3.4`
- legacy version history: `v0.1.1` through `v1.3.4` are preserved
  pre-discipline provenance
- tag scheme: `vX.Y.Z`

Strict compatibility guarantees are prospective from the adoption boundary and
the `v1.4.0` baseline. Earlier tags remain real historical releases, but they
are not retroactively certified as having followed the current SemVer policy.

## Established compatibility contract at v1.4.0

### User workflows

Fathrs manages explicitly configured filesystem entries through four maintained
operations:

- `link`: make destinations match configured symlink/copy state;
- `unlink`: remove only destinations that still match configured managed state;
- `validate`: validate configuration semantics without filesystem mutation;
- `probe`: inspect whether destinations match configured desired state.

### CLI and invocation

The maintained executable name is `fathrs`.

The stable command surface includes:

- global `--config <PATH>`;
- global `--base-dir <PATH>`;
- `link [--force] [--dry-run]`;
- `unlink [--dry-run]`;
- `validate`;
- `probe [--warn-only]`.

Probe returns non-zero when drift is detected. Structured diagnostics go to
stderr; probe state lines are emitted on stdout.

### Configuration

The maintained configuration format is `links.toml`.

- top-level TOML tables are sections;
- section entries map source paths to destinations;
- simple entries may use a destination string;
- detailed entries may set `target`, `copy`, and `doas`;
- section-level `copy` and `doas` values are defaults;
- `sudo` remains a backward-compatible alias for `doas`;
- relative paths resolve under `--base-dir`, or the config directory when no
  base directory is supplied;
- `~` expands from `$HOME`.

Unknown fields in detailed entries are invalid.

### Persisted formats and state

Fathrs maintains no private database or hidden state format.

The durable user-authored input is `links.toml`. Managed destination files,
directories, copies, and symlinks are observable filesystem state rather than
an internal persistence format.

### Safety behavior

The v1.4.0 contract includes:

- complete plan validation before the first mutation;
- rejection of duplicate, identical, overlapping, and dangerous destinations;
- explicit `--force` for conflicting replacement;
- dry-run using the same plan as real execution;
- idempotent correct state;
- post-change verification;
- safe unlink refusal for drifted/unexpected destinations.

### Packaging and runtime

- Rust implementation;
- stable Rust toolchain;
- Unix/Linux target;
- binary name `fathrs`;
- ordinary operation does not require privilege escalation;
- entries configured with `doas = true` invoke `doas` for privileged
  filesystem operations.

There is no supported public Rust library API at this baseline.

## Retrofit adoption record

- reason for retrofit: the project already had a substantial release history
  before rigorous release governance became canonical Software Philosophy
- historical version range retained as provenance: `v0.1.1..v1.3.4`
- last historical release: `v1.3.4`
- audit basis for baseline: recovery/hardening pass, isolated filesystem test
  matrix, strict Clippy, rustdoc, install smoke tests, and formatting gate
- semantic delta from v1.3.4: backwards-compatible `unlink` capability,
  materially stronger `probe`/copy behavior, safety improvements, and fixes
- baseline classification: `minor`
- baseline version: `v1.4.0`
- history treatment: preserve all existing tags and commits; no retroactive
  normalization
- major-version decision: **no ceremonial v2.0.0**; no established compatibility
  break justifies one

## Release tooling

- commit convention: Conventional Commits prospectively from the SemVer adoption
  boundary
- changelog: `CHANGELOG.md`, future release entries generated from post-baseline
  history with git-cliff and semantically reviewed by the director
- version mutator/orchestrator: local `cargo-release`
- tag mechanism: local annotated `vX.Y.Z` tags created by cargo-release
- push policy: release commits/tags are inspected locally, then pushed explicitly
- crates.io publication: disabled
- hosted-release mechanism: optional GitHub Release used as a hosting surface;
  it is not the versioning authority
- artifact build: local repository recipe

## Project-specific deviations

None.

The general doctrine is defined by Software Philosophy's
MVP-gated/versioning discipline. This file records only Fathrs-specific state.
