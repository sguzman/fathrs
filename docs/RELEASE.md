# Release policy

Fathrs uses rigorous Semantic Versioning prospectively from the `v1.4.0`
retrofit baseline.

See [VERSIONING.md](./VERSIONING.md) for the compatibility contract, adoption
boundary, legacy-history treatment, and baseline rationale.

## Authority

Release classification and release mutation are **local repository operations**.

GitHub CI is evidence only. It may verify commits, but it does not choose a
version, edit Cargo metadata, create the release commit, create the version tag,
or publish a release.

The local release stack is:

- Conventional Commits for maintained post-adoption history;
- git-cliff for release-note generation;
- cargo-release for Cargo version mutation, release commit, and local annotated
  tag creation.

Release-only prerequisites are `git-cliff` and `cargo-release`. They are local
tools, not hosted release authority.

GitHub is an optional hosting surface for the pushed tag, release notes, and a
locally built archive.

## Semantic classification

Versions describe change to the established compatibility contract, not amount
of work.

- major: an established CLI/config/runtime/install contract becomes
  incompatible;
- minor: meaningful backwards-compatible capability;
- patch: backwards-compatible correction, performance, reliability, packaging,
  or dependency maintenance;
- no release: internal-only docs/tests/CI/refactor/style/chore/build work when no
  shipped contract warrants a release.

The director inspects the complete delta since the previous release and chooses
the highest actual semantic impact. Commit prefixes are evidence, not authority.

## Retrofit baseline: v1.4.0

Existing tags `v0.1.1` through `v1.3.4` are preserved as legacy
pre-discipline history.

`v1.4.0` is the first release governed by the current SemVer discipline.

The minor baseline is semantically justified by backwards-compatible new user
capability (`unlink` plus materially expanded probe/copy behavior) together
with fixes and hardening. The adoption of release discipline itself does not
cause the bump.

Do not reset history to `v1.0.0`, move old tags, or create a ceremonial
`v2.0.0`.

### Baseline bootstrap

The baseline version and changelog are already prepared in the repository.

The retrofit ceremony is **one-time only** and is repository-owned:

```fish
just semver-baseline
```

That recipe requires `main`, requires a clean working tree, runs `just ci`,
creates the annotated `v1.4.0` baseline tag if needed, and pushes `main` plus
that tag. It is deliberately idempotent only when an existing `v1.4.0` tag
already points at the same commit; it refuses a conflicting historical tag.

After `v1.4.0` exists, never perform the retrofit ceremony again. Ordinary
future releases use the normal `release-dry-run` / `release` recipes below.

A hosted GitHub Release and locally built archive remain optional distribution
surfaces; neither is versioning authority.

## Subsequent releases

Normal releases require the baseline tag to exist.

Choose the semantic level from the actual accumulated delta, then dry-run:

```bash
just release-dry-run patch
# or: minor / major
```

When the dry-run and evidence are correct:

```bash
just release patch
```

`cargo-release` then:

1. updates Cargo version metadata;
2. runs the git-cliff pre-release hook over **unreleased commits since the
   latest tag**;
3. stages the changelog;
4. creates `chore(release): prepare X.Y.Z`;
5. creates a local annotated `vX.Y.Z` tag;
6. does **not** publish to crates.io;
7. does **not** push automatically.

Inspect the result, then explicitly push the release commit and tag.

The changelog hook uses git-cliff's `--unreleased --tag ... --prepend` flow so
legacy pre-baseline history is never regenerated into future release notes.

## Verification

A release candidate must pass:

```bash
just ci
```

This includes locked Cargo check, strict Clippy, tests, rustdoc, and rustfmt.

For changes whose strongest evidence is real filesystem behavior rather than
mechanical tests, obtain that evidence before tagging.

## Local artifact

Build the Linux archive locally:

```bash
just release-artifact
```

The archive is written under `dist/` and is derived from the current Cargo
version.

Release artifacts must correspond to the exact tagged source.
