use std::env;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "fathrs", version, about = "Small declarative dotfile deployer")]
pub(crate) struct Args {
  #[command(subcommand)]
  pub(crate) command: Option<Command>,

  /// Path to links.toml.
  #[arg(long, default_value = "links.toml", global = true)]
  pub(crate) config: PathBuf,

  /// Base directory used to resolve relative paths. Defaults to the config directory.
  #[arg(long, global = true)]
  pub(crate) base_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
  /// Create configured symlinks and copies.
  Link {
    /// Replace conflicting existing destinations.
    #[arg(long)]
    force: bool,

    /// Print the planned changes without modifying the filesystem.
    #[arg(long)]
    dry_run: bool,
  },

  /// Validate the configuration without changing the filesystem.
  Validate,

  /// Report whether configured destinations match the desired state.
  Probe {
    /// Only emit drift/warning output.
    #[arg(long)]
    warn_only: bool,
  },
}

pub(crate) fn expand_home_path(path: &Path) -> PathBuf {
  let Some(path_str) = path.to_str() else {
    return path.to_path_buf();
  };

  if path_str == "~" {
    return home_directory().unwrap_or_else(|| path.to_path_buf());
  }

  if let Some(rest) = path_str.strip_prefix("~/") {
    if let Some(home) = home_directory() {
      return home.join(rest);
    }
  }

  path.to_path_buf()
}

fn home_directory() -> Option<PathBuf> {
  env::var_os("HOME").map(PathBuf::from)
}
