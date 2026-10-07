# Architecture

Fathrs deliberately has a small execution model.

## Modules

- `src/cli.rs`: command-line parsing and home-directory expansion.
- `src/main.rs`: configuration parsing, semantic validation, whole-plan
  construction, command dispatch, and probe summaries.
- `src/link.rs`: resolved plan entries, filesystem comparison, mutation,
  privilege escalation, and post-change verification.

## Execution invariants

### Plan before mutation

`link` and `probe` first construct the complete list of resolved
`PlanEntry` values.

During that phase Fathrs:

1. expands home-relative paths;
2. resolves relative paths under the selected base directory;
3. normalizes lexical `.` and `..` components;
4. rejects identical or overlapping source/destination pairs;
5. rejects dangerous destination roots;
6. rejects duplicate resolved destinations;
7. verifies every source is inspectable.

Only after every entry passes does `link` begin mutating the filesystem.

### One plan for dry and real execution

`link --dry-run` and a real `link` consume the same resolved plan and the
same destination-state checks. Dry-run exits before removal, directory
creation, copy, or symlink creation.

### Idempotency

A correct symlink is a no-op.

A copy entry recursively compares file type, directory membership, symlink
targets, file size, and file bytes. A matching copy is also a no-op.

### Post-change verification

After each real change, Fathrs probes the destination again. The command fails
if the resulting state does not match the desired state.

## Privilege escalation

The canonical config key is `doas`. `sudo` remains a serde alias only for
backward compatibility.

When an entry has `doas = true`, privileged filesystem operations are
performed by invoking `doas` with standard Unix commands. Fathrs itself does
not become a privileged process.

## Scope constraints

Do not add a templating language, secrets subsystem, package manager, plugin
runtime, host-profile engine, or repository synchronization layer to the core
tool. Those concerns belong elsewhere.

A proposed feature should directly improve safe deployment, inspection, or
removal of explicitly configured filesystem entries.
