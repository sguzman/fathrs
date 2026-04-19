use std::env;
use std::path::{
  Path,
  PathBuf
};

use clap::Parser;


#[derive(Parser, Debug)]
#[command(
  name = "dotlink",
  version,
  about = "Simple dotfile linker: \
           links.toml -> symlinks"
)]
pub(crate) struct Args {
  #[command(subcommand)]
  pub(crate) command: Option<Command>,

  /// Path to links.toml
  #[arg(
    long,
    default_value = "links.toml",
    global = true
  )]
  pub(crate) config: PathBuf,

  /// Base directory used to resolve
  /// relative paths in links.toml
  /// (defaults to directory of config)
  #[arg(long, global = true)]
  pub(crate) base_dir: Option<PathBuf>
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Command {
  /// Create symlinks
  Link {
    /// Replace existing targets
    #[arg(long)]
    force: bool,

    /// Print what would happen, but do
    /// not modify filesystem
    #[arg(long)]
    dry_run: bool
  },

  /// Validate TOML format and schema
  Validate,

  /// Probe status of links
  Probe {
    /// Only emit warnings
    #[arg(long)]
    warn_only: bool
  }
}

pub(crate) fn expand_home_path(
  path: &Path
) -> PathBuf {
  let path_str = match path.to_str() {
    | Some(s) => s,
    | None => return path.to_path_buf()
  };

  if path_str == "~" {
    return home_directory()
      .unwrap_or_else(|| {
        path.to_path_buf()
      });
  }

  if let Some(rest) =
    path_str.strip_prefix("~/")
  {
    if let Some(home) = home_directory()
    {
      return home.join(rest);
    }
  }

  path.to_path_buf()
}

fn home_directory() -> Option<PathBuf> {
  env::var_os("HOME").map(PathBuf::from)
}
