# Fathrs Roadmap

Fathrs should be a small, dependable dotfile deployer: a declarative `links.toml`
describes source-to-destination mappings, and Fathrs makes the machine match that
description with symlinks or explicit copies.

The goal is not to recreate Dotter. Fathrs should stay narrow, inspectable, and
safe enough to use as the owner's real dotfile manager.

## Design Contract

Fathrs should remain:

- Rust-native and Unix/Linux-focused.
- Declarative: plain TOML in, filesystem operations out.
- Small: no templating language, plugin system, profile engine, secret manager,
  package manager, or repository mutation framework.
- Predictable: a dry run should accurately describe the real run.
- Idempotent: rerunning a correct configuration should be a no-op.
- Conservative with destructive operations.
- Easy to install and easy to verify.

## Phase 0 - Establish a trustworthy baseline

Before adding features, make HEAD coherent and prove what currently works.

- [ ] Add normal CI for pushes and pull requests.
- [ ] Make the CI toolchain match the repository toolchain.
- [ ] Reduce CI to tools that actually belong to this project and are
      installable in the declared environment.
- [ ] Remove stale template tasks from the `justfile` (Cite Otter fixture and
      normalization commands, unrelated scripts, etc.).
- [ ] Make `just ci` the authoritative local verification command.
- [ ] Run/build/test the current program through CI and record/fix every failure.
- [ ] Replace the obsolete generic migration roadmap with project-specific
      documentation once its useful historical material has been checked.

Acceptance criterion: a clean checkout has one obvious verification path and
that path passes on Linux.

## Phase 1 - Fix known correctness bugs

These are known defects in current HEAD and should be repaired before Fathrs is
used on real dotfiles.

- [ ] Fix absolute `--base-dir` handling. An absolute base directory must be
      used as supplied rather than discarded in favor of the config directory.
- [ ] Fix privileged copy mode to use the same privilege mechanism as the rest
      of the application (`doas`, not the leftover `sudo cp -r`).
- [ ] Fix CLI identity: Clap still calls the program `dotlink`; all user-facing
      names should be `fathrs`.
- [ ] Fix the stale tracing target/default filter that still refers to
      `dotlink`.
- [ ] Fix integration tests so they exercise the current subcommand structure
      rather than an obsolete CLI shape.
- [ ] Audit path normalization and relative symlink comparison for edge cases.
- [ ] Audit copy semantics for files, directories, symlinks, and replacement.
- [ ] Make schema, parser behavior, examples, and README describe exactly the
      same configuration language.

Acceptance criterion: the documented CLI and configuration examples work
exactly as written.

## Phase 2 - Make destructive behavior safe

`--force` currently has enough authority to recursively remove an existing
destination directory. That is too dangerous to trust with a real home
directory until the execution plan is validated first.

- [ ] Introduce a preflight/planning pass: resolve and validate every operation
      before mutating the filesystem.
- [ ] Reject obviously dangerous or nonsensical mappings, including source and
      destination resolving to the same path.
- [ ] Ensure missing/invalid sources fail before any earlier entries are changed.
- [ ] Define explicit replacement rules for files, directories, and symlinks.
- [ ] Make `--dry-run` use the same plan as the real execution path so it cannot
      disagree about what will happen.
- [ ] Make `--force` output identify exactly what will be removed/replaced.
- [ ] Add regression tests around recursive directory replacement and other
      destructive cases.
- [ ] Decide whether a lightweight backup option is warranted; do not add one
      unless it can remain simple and deterministic.

Acceptance criterion: a bad configuration cannot partially mutate earlier
entries before Fathrs discovers a predictable validation error later in the
file.

## Phase 3 - Build a serious test matrix

The existing integration coverage is far too small for software that rewrites
filesystem state.

- [ ] Unit-test path expansion and resolution (`~`, relative paths, absolute
      paths, `.`, `..`).
- [ ] Test file symlinks and directory symlinks.
- [ ] Test idempotent reruns.
- [ ] Test conflicts with and without `--force`.
- [ ] Test wrong-target symlink replacement.
- [ ] Test broken symlinks.
- [ ] Test copy mode for files and directory trees.
- [ ] Test section defaults and per-entry overrides for `copy` and privilege
      escalation.
- [ ] Test `probe` and `--warn-only`.
- [ ] Test dry-run leaves the filesystem untouched.
- [ ] Test validation failures and useful error messages.
- [ ] Isolate privileged-operation construction so it can be tested without
      actually escalating privileges.
- [ ] Use temporary directories rather than mutating checked-in example
      directories during tests.

Acceptance criterion: ordinary filesystem behavior is comprehensively testable
without root/doas and CI covers all non-privileged behavior.

## Phase 4 - Clean the configuration model

Keep the format small, but make it unambiguous and stable.

- [ ] Decide final terminology for the privilege flag. The config currently says
      `sudo = true` while the implementation uses `doas`.
- [ ] Preserve backward compatibility if renaming that flag is worthwhile.
- [ ] Tighten the JSON schema so malformed detailed entries and unknown fields
      are caught where appropriate.
- [ ] Make `validate` perform meaningful semantic validation in addition to TOML
      deserialization.
- [ ] Produce clear diagnostics containing section, source, destination, and the
      failed rule.
- [ ] Document path-resolution rules once, with tests mirroring the examples.

Acceptance criterion: users can tell whether a config is valid without touching
the filesystem.

## Phase 5 - Make probe/status genuinely useful

Probe should answer "is this machine in the state my dotfiles repo describes?"

- [ ] Compare a symlink's resolved target with the configured source, not merely
      report that a symlink exists.
- [ ] For copy entries, distinguish missing, present-and-matching, and drifted.
- [ ] Give probe a stable concise summary and non-zero exit status when drift is
      detected.
- [ ] Keep `--warn-only` useful for shell/login checks.
- [ ] Separate actual privilege requirements from the configured preference to
      use privilege escalation.

Acceptance criterion: `fathrs probe` can be used as a reliable machine-state
check.

## Phase 6 - Installation and release hygiene

Once behavior is trustworthy, make installation boring.

- [ ] Bring the crate version, changelog, tags, and actual feature history back
      into sync.
- [ ] Repair stale `sguzman/fathers` links and other old project-name residue.
- [ ] Replace deprecated/broken release workflow pieces.
- [ ] Ensure release artifacts are built only after the same checks used by CI.
- [ ] Decide on the supported Rust channel and use it consistently.
- [ ] Verify `cargo install --git https://github.com/sguzman/fathrs` and
      `cargo install --path .` workflows.
- [ ] Create a real release only after the repaired build is proven.

Acceptance criterion: a tagged release corresponds to tested source and yields a
working `fathrs` binary on the intended Linux environment.

## Phase 7 - Daily-use dotfile ergonomics

Only after the core is safe and tested, add the small conveniences that make it
pleasant as the owner's actual dotfile manager.

- [ ] Create a realistic dotfiles fixture modeled on the intended EndeavourOS /
      Hyprland setup without committing private machine data.
- [ ] Ensure one config can comfortably mix linked user config, copied files, and
      privileged system destinations.
- [ ] Consider an explicit `unlink`/remove-managed-links command, with the same
      plan-first safety model.
- [ ] Consider a concise machine-readable probe format only if another local
      tool actually needs it.
- [ ] Improve output readability without hiding filesystem actions.

Acceptance criterion: the owner's real dotfiles can be described cleanly
without adding framework machinery.

## Phase 8 - Documentation cleanup

Documentation should describe the tool, not the history of AI/template work
around it.

- [ ] Keep the README focused on purpose, install, config, common commands, and
      safety behavior.
- [ ] Remove or archive irrelevant template/reference documentation.
- [ ] Keep a short architecture/developer document for the execution model and
      invariants.
- [ ] Keep changelog/release documentation accurate and generated consistently.

Acceptance criterion: a new user can understand and safely use Fathrs from the
README alone.

## Execution Order

Work through the phases in order. Within the first pass, the priority is:

1. Baseline CI/tooling.
2. Known correctness bugs.
3. Destructive-operation safety.
4. Test matrix.
5. Config validation.
6. Probe correctness.
7. Release/install cleanup.
8. Daily-use conveniences and documentation polish.

No major new features should be added until Phases 0-3 are green.
