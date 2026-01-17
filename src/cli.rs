use std::env;
use std::path::{
  Path,
  PathBuf
};

use clap::Parser;

/// Path to the JSON schema that
/// documents valid `links.toml` files.
pub const LINKS_SCHEMA_PATH: &str =
  "schema/links.schema.json";

#[derive(Parser, Debug)]
#[command(
  name = "dotlink",
  version,
  about = "Simple dotfile linker: \
           links.toml -> symlinks"
)]
pub(crate) struct Args {
  /// Path to links.toml
  #[arg(
    long,
    default_value = "links.toml"
  )]
  pub(crate) config: PathBuf,

  /// Base directory used to resolve
  /// relative paths in links.toml
  /// (defaults to directory of config)
  #[arg(long)]
  pub(crate) base_dir: Option<PathBuf>,

  /// Replace existing targets (remove
  /// file/dir/symlink at target and
  /// recreate)
  #[arg(long)]
  pub(crate) force: bool,

  /// Print what would happen, but do
  /// not modify filesystem
  #[arg(long)]
  pub(crate) dry_run: bool,

  /// Verify each link’s status instead
  /// of creating links
  #[arg(long)]
  pub(crate) status: bool
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
