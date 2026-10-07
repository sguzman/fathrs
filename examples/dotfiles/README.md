# Realistic dotfiles example

This fixture models the intended shape of a small Linux dotfiles repository.

It demonstrates:

- linked user configuration directories;
- one generated configuration copied into place;
- one privileged systemd unit deployed with `doas`.

Preview it from this directory with:

```bash
fathrs --config links.toml link --dry-run
```

Do not apply the example unchanged on a real machine: its destinations are
illustrative.
