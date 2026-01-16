use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{
  Path,
  PathBuf
};
use std::{
  fs,
  io
};

use anyhow::{
  Context,
  Result,
  anyhow,
  bail
};
use clap::Parser;
#[cfg(unix)]
use libc;
use serde::Deserialize;
use tracing::{
  debug,
  error,
  info,
  instrument,
  trace,
  warn
};
use tracing_subscriber::EnvFilter;

/// Location of the JSON schema that
/// describes valid `links.toml` files.
const LINKS_SCHEMA_PATH: &str =
  "schema/links.schema.json";

#[derive(Parser, Debug)]
#[command(
  name = "dotlink",
  version,
  about = "Simple dotfile linker: \
           links.toml -> symlinks"
)]
struct Args {
  /// Path to links.toml
  #[arg(
    long,
    default_value = "links.toml"
  )]
  config: PathBuf,

  /// Base directory used to resolve
  /// relative paths in links.toml
  /// (defaults to directory of config)
  #[arg(long)]
  base_dir: Option<PathBuf>,

  /// Replace existing targets (remove
  /// file/dir/symlink at target and
  /// recreate)
  #[arg(long)]
  force: bool,

  /// Print what would happen, but do
  /// not modify filesystem
  #[arg(long)]
  dry_run: bool,

  /// Verify each link’s status instead
  /// of creating links
  #[arg(long)]
  status: bool
}

// links.toml format:
// [section]
// "source/path" = "target/path"
//
// This deserializes into: section ->
// (source -> target)
#[derive(Debug, Deserialize)]
struct LinksToml(
  BTreeMap<
    String,
    BTreeMap<String, String>
  >
);

fn main() -> Result<()> {
  // Intense logging via tracing +
  // EnvFilter. Control verbosity with
  // RUST_LOG, e.g.:   RUST_LOG=trace
  // Default: info (but we log a lot at
  // debug/trace).
  tracing_subscriber::fmt()
    .with_env_filter(
      EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
          EnvFilter::new(
            "info,dotlink=trace"
          )
        })
    )
    .with_target(true)
    .with_level(true)
    .with_line_number(true)
    .with_file(true)
    .init();

  let args = Args::parse();

  if let Err(e) = run(args) {
    error!("{:#}", e);
    std::process::exit(1);
  }
  Ok(())
}

#[instrument(level = "trace", skip_all, fields(config=?args.config, force=args.force, dry_run=args.dry_run))]
fn run(args: Args) -> Result<()> {
  let config_path =
    fs::canonicalize(&args.config)
      .with_context(|| {
        format!(
          "failed to canonicalize \
           config path: {:?}",
          args.config
        )
      })?;

  let config_dir = config_path
    .parent()
    .ok_or_else(|| {
      anyhow!(
        "config file has no parent \
         dir: {:?}",
        config_path
      )
    })?
    .to_path_buf();

  let base_dir =
    args.base_dir.unwrap_or_else(
      || config_dir.clone()
    );
  info!(
    ?config_path,
    ?base_dir,
    schema = %LINKS_SCHEMA_PATH,
    "starting dotlink"
  );

  let raw =
    fs::read_to_string(&config_path)
      .with_context(|| {
        format!(
          "failed to read config \
           file: {:?}",
          config_path
        )
      })?;

  let links: LinksToml =
    toml::from_str(&raw).with_context(
      || {
        format!(
          "failed to parse TOML in \
           {:?}",
          config_path
        )
      }
    )?;

  let mut total = 0usize;
  for (section, mapping) in &links.0 {
    info!(section = %section, count = mapping.len(), "processing section");
    for (src_str, dst_str) in mapping {
      total += 1;
      link_one(
        &base_dir,
        Path::new(src_str),
        Path::new(dst_str),
        args.force,
        args.dry_run,
        args.status,
        section
      )
      .with_context(|| {
        format!(
          "section={section} \
           src={src_str} dst={dst_str}"
        )
      })?;
    }
  }

  info!(total, "done");
  Ok(())
}

#[instrument(
    level="trace",
    skip_all,
    fields(section=%section, force=force, dry_run=dry_run, status=status, src=%src.display(), dst=%dst.display())
)]
fn link_one(
  base_dir: &Path,
  src: &Path,
  dst: &Path,
  force: bool,
  dry_run: bool,
  status: bool,
  section: &str
) -> Result<()> {
  let src_abs =
    resolve_under(base_dir, src);
  let dst_abs =
    resolve_under(base_dir, dst);

  debug!(src_abs=%src_abs.display(), dst_abs=%dst_abs.display(), "resolved paths");

  // Verify source exists
  let src_meta =
    fs::symlink_metadata(&src_abs)
      .with_context(|| {
        format!(
          "source path does not exist \
           or cannot be stat'd: {}",
          src_abs.display()
        )
      })?;
  let src_ft = src_meta.file_type();
  let src_is_dir = src_ft.is_dir();
  let src_is_file = src_ft.is_file();
  let src_is_symlink =
    src_ft.is_symlink();

  trace!(
    src_is_dir,
    src_is_file,
    src_is_symlink,
    "source file type"
  );

  if status {
    log_symlink_status(&dst_abs)?;
    log_sudo_requirement(&dst_abs);
    return Ok(());
  }

  // Ensure parent directories exist for
  // the destination path
  if let Some(parent) = dst_abs.parent()
  {
    if !parent.as_os_str().is_empty() {
      debug!(parent=%parent.display(), "ensuring destination parent exists");
      if !dry_run {
        fs::create_dir_all(parent)
          .with_context(|| {
            format!(
              "failed to create \
               destination parent \
               dirs: {}",
              parent.display()
            )
          })?;
      }
    }
  }

  // If destination exists, decide what
  // to do
  match fs::symlink_metadata(&dst_abs) {
        Ok(dst_meta) => {
            let dst_ft = dst_meta.file_type();
            debug!(
                dst_is_symlink = dst_ft.is_symlink(),
                dst_is_dir = dst_ft.is_dir(),
                dst_is_file = dst_ft.is_file(),
                "destination exists"
            );

            // If it's a symlink already and points to the same target, skip.
            if dst_ft.is_symlink() {
                match fs::read_link(&dst_abs) {
                    Ok(existing_target) => {
                        debug!(existing_target=%existing_target.display(), "destination is symlink; read_link ok");

                        // Compare after resolving relative-to-dst-parent (read_link may return relative path)
                        let normalized_existing = normalize_link_target(&dst_abs, &existing_target);
                        let normalized_wanted = src_abs.clone();

                        if path_eq_loose(&normalized_existing, &normalized_wanted) {
                            info!("already linked; skipping");
                            return Ok(());
                        } else {
                            warn!(
                                existing=%normalized_existing.display(),
                                wanted=%normalized_wanted.display(),
                                "symlink exists but points elsewhere"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(error=?e, "destination is symlink but read_link failed; treating as conflict");
                    }
                }
            }

            if !force {
                bail!(
                    "destination exists and --force not set: {}",
                    dst_abs.display()
                );
            }

            warn!("--force enabled; removing existing destination");
            if !dry_run {
                remove_any_path(&dst_abs).with_context(|| {
                    format!("failed removing existing destination: {}", dst_abs.display())
                })?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            trace!("destination does not exist; good");
        }
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "failed to symlink_metadata destination: {}",
                    dst_abs.display()
                )
            });
        }
    }

  // Create the symlink
  info!(
    kind = if src_is_dir {
      "dir"
    } else {
      "file"
    },
    "creating symlink"
  );

  if dry_run {
    info!(
      "dry-run: would create symlink"
    );
    return Ok(());
  }

  create_symlink(&src_abs, &dst_abs)
    .with_context(|| {
      format!(
        "failed to create symlink {} \
         -> {}",
        dst_abs.display(),
        src_abs.display()
      )
    })?;

  debug!("symlink created");

  // Post-check
  match fs::symlink_metadata(&dst_abs) {
    | Ok(md) => {
      debug!(
        is_symlink =
          md.file_type().is_symlink(),
        "post-check metadata"
      );
    }
    | Err(e) => {
      warn!(error=?e, "post-check failed");
    }
  }

  Ok(())
}

/// Resolve `p` under `base_dir` if it's
/// relative; otherwise return as-is.
/// Also normalizes `.` / `..` without
/// hitting the filesystem.
fn resolve_under(
  base_dir: &Path,
  p: &Path
) -> PathBuf {
  if p.is_absolute() {
    normalize_path(p)
  } else {
    normalize_path(&base_dir.join(p))
  }
}

/// Remove a file/dir/symlink at path,
/// without following symlinks into
/// directories. This removes *the path
/// itself*.
#[instrument(level="trace", skip_all, fields(path=%path.display()))]
fn remove_any_path(
  path: &Path
) -> Result<()> {
  let md = fs::symlink_metadata(path)?;
  let ft = md.file_type();

  if ft.is_symlink() || ft.is_file() {
    debug!("removing as file/symlink");
    fs::remove_file(path)?;
    return Ok(());
  }

  if ft.is_dir() {
    debug!(
      "removing as directory \
       (recursive)"
    );
    fs::remove_dir_all(path)?;
    return Ok(());
  }

  bail!(
    "unsupported file type at {}",
    path.display()
  );
}

/// Create symlink at `link_path`
/// pointing to `target`.
/// `target_is_dir` matters on Windows.
#[instrument(level="trace", skip_all, fields(target=%target.display(), link_path=%link_path.display()))]
fn create_symlink(
  target: &Path,
  link_path: &Path
) -> Result<()> {
  #[cfg(unix)]
  {
    // Unix: one symlink API, works for
    // files and dirs.
    std::os::unix::fs::symlink(
      target, link_path
    )?;
    return Ok(());
  }

  #[allow(unreachable_code)]
  Err(anyhow!(
    "unsupported platform for symlinks"
  ))
}

fn log_symlink_status(
  dst_abs: &Path
) -> Result<()> {
  match fs::symlink_metadata(dst_abs) {
    | Ok(meta) => {
      if meta.file_type().is_symlink() {
        info!(dst=%dst_abs.display(), "symlink already exists");
      } else {
        warn!(dst=%dst_abs.display(), "destination exists but is not a symlink");
      }
    }
    | Err(e)
      if e.kind()
        == io::ErrorKind::NotFound =>
    {
      warn!(dst=%dst_abs.display(), "symlink missing");
    }
    | Err(e) => {
      warn!(dst=%dst_abs.display(), error=?e, "failed to stat destination");
    }
  }
  Ok(())
}

fn log_sudo_requirement(
  dst_abs: &Path
) {
  if requires_sudo_for_path(dst_abs) {
    warn!(dst=%dst_abs.display(), "creating this link may require sudo");
  } else {
    info!(dst=%dst_abs.display(), "creating this link should not require sudo");
  }
}

#[cfg(unix)]
fn requires_sudo_for_path(
  path: &Path
) -> bool {
  let parent = path
    .parent()
    .unwrap_or_else(|| Path::new("/"));
  let parent =
    if parent.as_os_str().is_empty() {
      Path::new("/").to_path_buf()
    } else {
      parent.to_path_buf()
    };

  let mut cursor = parent;
  loop {
    if cursor.exists() {
      break;
    }
    if let Some(p) = cursor.parent() {
      cursor = p.to_path_buf();
      continue;
    }
    break;
  }

  if cursor.as_os_str().is_empty() {
    cursor =
      Path::new("/").to_path_buf();
  }

  let meta = match fs::metadata(&cursor)
  {
    | Ok(m) => m,
    | Err(_) => return true
  };

  let mode = meta.mode();
  let owner_uid = meta.uid();
  let owner_gid = meta.gid();
  let current_uid =
    unsafe { libc::geteuid() } as u32;
  let current_gid =
    unsafe { libc::getegid() } as u32;

  let writable =
    if owner_uid == current_uid {
      mode & 0o200 != 0
    } else if owner_gid == current_gid {
      mode & 0o020 != 0
    } else {
      mode & 0o002 != 0
    };

  !writable
}

#[cfg(not(unix))]
fn requires_sudo_for_path(
  _path: &Path
) -> bool {
  false
}

/// Normalize read_link output to an
/// absolute-ish path for comparison: If
/// read_link returns a relative target,
/// it's relative to the link's parent.
fn normalize_link_target(
  link_path: &Path,
  read_link_target: &Path
) -> PathBuf {
  if read_link_target.is_absolute() {
    normalize_path(read_link_target)
  } else {
    let parent = link_path
      .parent()
      .unwrap_or_else(|| {
        Path::new(".")
      });
    normalize_path(
      &parent.join(read_link_target)
    )
  }
}

/// Loose path equality: try
/// canonicalize if possible (target may
/// not exist for broken links),
/// otherwise compare normalized
/// strings.
fn path_eq_loose(
  a: &Path,
  b: &Path
) -> bool {
  match (
    fs::canonicalize(a),
    fs::canonicalize(b)
  ) {
    | (Ok(ac), Ok(bc)) => ac == bc,
    | _ => {
      normalize_path(a)
        == normalize_path(b)
    }
  }
}

/// Pure path normalization (no
/// filesystem access): remove `.` and
/// resolve `..` lexically.
fn normalize_path(p: &Path) -> PathBuf {
  use std::path::Component;

  let mut out = PathBuf::new();
  for c in p.components() {
    match c {
      | Component::CurDir => {}
      | Component::ParentDir => {
        // Pop only if possible; if we
        // can't, keep the .. (important
        // for relative paths)
        if !out.pop() {
          out.push("..");
        }
      }
      | other => {
        out.push(other.as_os_str())
      }
    }
  }
  out
}
