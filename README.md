# fathrs

`fathrs` is a small Rust CLI for deploying dotfiles and other filesystem
configuration from a declarative `links.toml`.

It does two things:

- create symlinks;
- copy files or directory trees when an actual copy is required.

That is intentionally the whole scope. Fathrs is not a templating engine, secret
manager, package manager, profile framework, or repository manager.

## Status

Fathrs is being hardened for real daily use on Linux. The current execution
model validates the complete configuration before changing the filesystem,
supports dry runs, verifies changes after applying them, and can probe for drift.

See [ROADMAP.md](./ROADMAP.md) for remaining hardening and release work.

## Install

From a local checkout:

```bash
cargo install --path .
```

From GitHub:

```bash
cargo install --git https://github.com/sguzman/fathrs
```

The project targets stable Rust and Unix/Linux filesystems.

Privileged entries use `doas`. Non-privileged configurations do not require it.

## Quick start

Create `links.toml` next to the files you want to manage:

```toml
[dotfiles]
"fish" = "~/.config/fish"
"nvim" = "~/.config/nvim"
"hypr" = "~/.config/hypr"
```

Validate it:

```bash
fathrs --config ./links.toml validate
```

Preview the deployment:

```bash
fathrs --config ./links.toml link --dry-run
```

Apply it:

```bash
fathrs --config ./links.toml link
```

If a destination already exists and conflicts with the desired state, Fathrs
refuses to replace it unless `--force` is supplied:

```bash
fathrs --config ./links.toml link --force
```

Check whether the machine still matches the configuration:

```bash
fathrs --config ./links.toml probe
```

`probe` exits non-zero when it finds drift.

## Configuration

Every top-level table is a section. Entries map a source path to a destination.

Simple symlink entries:

```toml
[user]
".gitconfig" = "~/.gitconfig"
"kitty" = "~/.config/kitty"
```

Section defaults can select copy mode or privilege escalation:

```toml
[system]
doas = true
"system/example.service" = "/etc/systemd/system/example.service"

[copies]
copy = true
"generated/tool.conf" = "~/.config/tool/tool.conf"
```

An individual entry can override section defaults:

```toml
[mixed]
copy = true
"snapshot.conf" = "~/.config/example/snapshot.conf"
"live-dir" = { target = "~/.config/example/live-dir", copy = false }
"system.conf" = { target = "/etc/example.conf", copy = true, doas = true }
```

The legacy key `sudo = true` is accepted as an alias for `doas = true` so old
configs continue to parse. New configs should use `doas`.

### Path resolution

- `--config` defaults to `links.toml`.
- `~` and `~/...` expand using `$HOME`.
- Absolute paths are used as written.
- Relative source and destination paths resolve under `--base-dir`.
- Without `--base-dir`, relative paths resolve under the directory containing
  `links.toml`.

This makes a dotfiles repository self-contained by default.

## Commands

### `link`

Applies the validated plan.

- `--dry-run`: show the same planned operations without changing the
  filesystem.
- `--force`: replace conflicting destinations.

Correct symlinks and matching copies are idempotent and skipped.

### `validate`

Parses the TOML and performs semantic checks without requiring source files to
exist or changing the filesystem. It rejects empty configurations, duplicate
destinations, identical/overlapping source and destination paths, and dangerous
top-level destinations.

### `probe`

Compares configured destinations with their sources.

It distinguishes:

- `OK`
- `MISSING`
- `WRONG-KIND`
- `WRONG-TARGET`
- `DRIFTED` for copy entries whose content differs

Use `--warn-only` to suppress `OK` lines.

## Safety model

Fathrs resolves and validates every entry before the first mutation. A later
missing source therefore cannot leave earlier entries partially deployed.

Other safety rules include:

- duplicate resolved destinations are rejected;
- source and destination cannot resolve to the same path;
- source and destination cannot contain one another;
- filesystem root, top-level system directories, and the home directory itself
  are rejected as direct destinations;
- conflicting destinations require explicit `--force`;
- every applied operation is probed again afterward.

`--force` can remove a conflicting destination, including a directory tree.
Use `link --dry-run` first when changing a real dotfiles deployment.

## Development

The local verification contract is:

```bash
just ci
```

It runs Cargo check, Clippy with warnings denied, the test suite, rustdoc, and
the formatting gate.

Useful commands:

```bash
just test
just clippy
just fmt
just build
just install
```

CI runs the same `just ci` contract on pushes to `main` and pull requests.

See [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for execution invariants and
[docs/RELEASE.md](./docs/RELEASE.md) for release policy.

## License

MIT.
