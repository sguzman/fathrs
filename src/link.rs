use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, BufReader, Read};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use tracing::{info, warn};

use crate::cli::expand_home_path;

#[derive(Debug)]
pub(crate) struct PlanEntry {
  pub(crate) section: String,
  pub(crate) src: PathBuf,
  pub(crate) dst: PathBuf,
  pub(crate) use_doas: bool,
  pub(crate) use_copy: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProbeState {
  Ok,
  Missing,
  WrongKind,
  WrongTarget,
  Drifted,
}

impl ProbeState {
  pub(crate) fn label(self) -> &'static str {
    match self {
      Self::Ok => "OK",
      Self::Missing => "MISSING",
      Self::WrongKind => "WRONG-KIND",
      Self::WrongTarget => "WRONG-TARGET",
      Self::Drifted => "DRIFTED",
    }
  }
}

impl PlanEntry {
  pub(crate) fn new(
    base_dir: &Path,
    section: &str,
    src: &Path,
    dst: &Path,
    use_doas: bool,
    use_copy: bool,
  ) -> Result<Self> {
    Ok(Self {
      section: section.to_owned(),
      src: resolve_under(base_dir, src),
      dst: resolve_under(base_dir, dst),
      use_doas,
      use_copy,
    })
  }

  pub(crate) fn validate_static(&self) -> Result<()> {
    if self.src == self.dst {
      bail!(
        "section {:?}: source and destination resolve to the same path: {}",
        self.section,
        self.src.display()
      );
    }

    if self.src.starts_with(&self.dst) || self.dst.starts_with(&self.src) {
      bail!(
        "section {:?}: source and destination must not contain one another: {} <-> {}",
        self.section,
        self.src.display(),
        self.dst.display()
      );
    }

    if dangerous_destination(&self.dst) {
      bail!(
        "section {:?}: refusing dangerous destination path: {}",
        self.section,
        self.dst.display()
      );
    }

    Ok(())
  }

  pub(crate) fn validate_source(&self) -> Result<()> {
    fs::symlink_metadata(&self.src).with_context(|| {
      format!(
        "section {:?}: source path does not exist or cannot be inspected: {}",
        self.section,
        self.src.display()
      )
    })?;
    Ok(())
  }

  pub(crate) fn write_requires_privilege(&self) -> bool {
    requires_privilege_for_path(&self.dst)
  }
}

pub(crate) fn apply_entry(entry: &PlanEntry, force: bool, dry_run: bool) -> Result<()> {
  let state = probe_entry(entry)?;

  if state == ProbeState::Ok {
    info!(
      section = %entry.section,
      dst = %entry.dst.display(),
      "already in desired state; skipping"
    );
    return Ok(());
  }

  let destination_exists = state != ProbeState::Missing;
  if destination_exists && !force {
    bail!(
      "section {:?}: destination conflicts with desired state ({}); use --force to replace it: {}",
      entry.section,
      state.label(),
      entry.dst.display()
    );
  }

  if destination_exists {
    warn!(
      section = %entry.section,
      dst = %entry.dst.display(),
      state = state.label(),
      "replacing existing destination"
    );
  }

  let action = if entry.use_copy { "copy" } else { "symlink" };
  info!(
    section = %entry.section,
    src = %entry.src.display(),
    dst = %entry.dst.display(),
    action,
    doas = entry.use_doas,
    dry_run,
    "planned filesystem change"
  );

  if dry_run {
    return Ok(());
  }

  if destination_exists {
    remove_any_path(&entry.dst, entry.use_doas)
      .with_context(|| format!("failed to remove {}", entry.dst.display()))?;
  }

  if let Some(parent) = entry.dst.parent()
    && !parent.as_os_str().is_empty()
  {
    ensure_dir_all(parent, entry.use_doas)
      .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
  }

  if entry.use_copy {
    copy_any_path(&entry.src, &entry.dst, entry.use_doas)?;
  } else {
    create_symlink(&entry.src, &entry.dst, entry.use_doas)?;
  }

  let final_state = probe_entry(entry)?;
  if final_state != ProbeState::Ok {
    bail!(
      "post-change verification failed for {}: {}",
      entry.dst.display(),
      final_state.label()
    );
  }

  Ok(())
}

pub(crate) fn unlink_entry(entry: &PlanEntry, dry_run: bool) -> Result<()> {
  let state = probe_entry(entry)?;

  match state {
    ProbeState::Missing => {
      info!(
        section = %entry.section,
        dst = %entry.dst.display(),
        "destination already absent; skipping"
      );
      return Ok(());
    }
    ProbeState::Ok => {}
    other => {
      bail!(
        "section {:?}: refusing to unlink destination in state {}: {}",
        entry.section,
        other.label(),
        entry.dst.display()
      );
    }
  }

  info!(
    section = %entry.section,
    dst = %entry.dst.display(),
    dry_run,
    "planned managed destination removal"
  );

  if dry_run {
    return Ok(());
  }

  remove_any_path(&entry.dst, entry.use_doas)
    .with_context(|| format!("failed to remove {}", entry.dst.display()))?;

  if fs::symlink_metadata(&entry.dst).is_ok() {
    bail!(
      "post-unlink verification failed; destination still exists: {}",
      entry.dst.display()
    );
  }

  Ok(())
}

pub(crate) fn probe_entry(entry: &PlanEntry) -> Result<ProbeState> {
  let dst_meta = match fs::symlink_metadata(&entry.dst) {
    Ok(meta) => meta,
    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ProbeState::Missing),
    Err(error) => {
      return Err(error)
        .with_context(|| format!("failed to inspect destination {}", entry.dst.display()));
    }
  };

  if entry.use_copy {
    return Ok(if paths_match(&entry.src, &entry.dst)? {
      ProbeState::Ok
    } else {
      ProbeState::Drifted
    });
  }

  if !dst_meta.file_type().is_symlink() {
    return Ok(ProbeState::WrongKind);
  }

  let target = fs::read_link(&entry.dst)
    .with_context(|| format!("failed to read symlink {}", entry.dst.display()))?;
  let resolved_target = normalize_link_target(&entry.dst, &target);

  Ok(if path_eq_loose(&resolved_target, &entry.src) {
    ProbeState::Ok
  } else {
    ProbeState::WrongTarget
  })
}

pub(crate) fn resolve_under(base_dir: &Path, path: &Path) -> PathBuf {
  let expanded = expand_home_path(path);
  if expanded.is_absolute() {
    normalize_path(&expanded)
  } else {
    normalize_path(&base_dir.join(expanded))
  }
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
  let mut out = PathBuf::new();

  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        if !out.pop() {
          out.push("..");
        }
      }
      other => out.push(other.as_os_str()),
    }
  }

  out
}

fn dangerous_destination(path: &Path) -> bool {
  if path == Path::new("/") {
    return true;
  }

  if path.parent() == Some(Path::new("/")) {
    return true;
  }

  if let Some(home) = env::var_os("HOME").map(PathBuf::from)
    && normalize_path(path) == normalize_path(&home)
  {
    return true;
  }

  false
}

fn normalize_link_target(link_path: &Path, target: &Path) -> PathBuf {
  if target.is_absolute() {
    normalize_path(target)
  } else {
    normalize_path(&link_path.parent().unwrap_or_else(|| Path::new(".")).join(target))
  }
}

fn path_eq_loose(a: &Path, b: &Path) -> bool {
  match (fs::canonicalize(a), fs::canonicalize(b)) {
    (Ok(a), Ok(b)) => a == b,
    _ => normalize_path(a) == normalize_path(b),
  }
}

fn paths_match(src: &Path, dst: &Path) -> Result<bool> {
  let src_meta = match fs::symlink_metadata(src) {
    Ok(meta) => meta,
    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
    Err(error) => return Err(error.into()),
  };
  let dst_meta = match fs::symlink_metadata(dst) {
    Ok(meta) => meta,
    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
    Err(error) => return Err(error.into()),
  };

  let src_type = src_meta.file_type();
  let dst_type = dst_meta.file_type();

  if src_type.is_symlink() || dst_type.is_symlink() {
    if !(src_type.is_symlink() && dst_type.is_symlink()) {
      return Ok(false);
    }
    return Ok(fs::read_link(src)? == fs::read_link(dst)?);
  }

  if src_type.is_file() || dst_type.is_file() {
    if !(src_type.is_file() && dst_type.is_file()) || src_meta.len() != dst_meta.len() {
      return Ok(false);
    }
    return files_equal(src, dst);
  }

  if src_type.is_dir() || dst_type.is_dir() {
    if !(src_type.is_dir() && dst_type.is_dir()) {
      return Ok(false);
    }

    let src_names = directory_names(src)?;
    let dst_names = directory_names(dst)?;
    if src_names != dst_names {
      return Ok(false);
    }

    for name in src_names {
      if !paths_match(&src.join(&name), &dst.join(&name))? {
        return Ok(false);
      }
    }

    return Ok(true);
  }

  Ok(false)
}

fn directory_names(path: &Path) -> Result<BTreeSet<std::ffi::OsString>> {
  let mut names = BTreeSet::new();
  for entry in fs::read_dir(path)? {
    names.insert(entry?.file_name());
  }
  Ok(names)
}

fn files_equal(a: &Path, b: &Path) -> Result<bool> {
  const BUFFER_SIZE: usize = 16 * 1024;

  let mut left = BufReader::new(fs::File::open(a)?);
  let mut right = BufReader::new(fs::File::open(b)?);
  let mut left_buf = [0_u8; BUFFER_SIZE];
  let mut right_buf = [0_u8; BUFFER_SIZE];

  loop {
    let left_len = left.read(&mut left_buf)?;
    let right_len = right.read(&mut right_buf)?;

    if left_len != right_len {
      return Ok(false);
    }
    if left_len == 0 {
      return Ok(true);
    }
    if left_buf[..left_len] != right_buf[..right_len] {
      return Ok(false);
    }
  }
}

fn ensure_dir_all(path: &Path, use_doas: bool) -> Result<()> {
  if path.exists() {
    return Ok(());
  }

  if use_doas {
    run_doas(["mkdir", "-p", "--"], Some(path))?;
  } else {
    fs::create_dir_all(path)?;
  }

  Ok(())
}

fn remove_any_path(path: &Path, use_doas: bool) -> Result<()> {
  let meta = fs::symlink_metadata(path)?;
  let file_type = meta.file_type();

  if use_doas {
    if file_type.is_dir() && !file_type.is_symlink() {
      run_doas(["rm", "-rf", "--"], Some(path))?;
    } else {
      run_doas(["rm", "-f", "--"], Some(path))?;
    }
    return Ok(());
  }

  if file_type.is_dir() && !file_type.is_symlink() {
    fs::remove_dir_all(path)?;
  } else {
    fs::remove_file(path)?;
  }

  Ok(())
}

fn create_symlink(target: &Path, link_path: &Path, use_doas: bool) -> Result<()> {
  if use_doas {
    let status = Command::new("doas")
      .arg("ln")
      .arg("-s")
      .arg("--")
      .arg(target)
      .arg(link_path)
      .status()
      .context("failed to execute doas ln")?;

    if !status.success() {
      bail!("doas ln -s failed with status {status}");
    }

    return Ok(());
  }

  #[cfg(unix)]
  {
    std::os::unix::fs::symlink(target, link_path)?;
    Ok(())
  }

  #[cfg(not(unix))]
  {
    let _ = (target, link_path);
    bail!("fathrs symlink mode is only supported on Unix")
  }
}

fn copy_any_path(src: &Path, dst: &Path, use_doas: bool) -> Result<()> {
  if use_doas {
    let status = Command::new("doas")
      .arg("cp")
      .arg("-a")
      .arg("--")
      .arg(src)
      .arg(dst)
      .status()
      .context("failed to execute doas cp")?;

    if !status.success() {
      bail!("doas cp -a failed with status {status}");
    }

    return Ok(());
  }

  copy_path_local(src, dst)
}

fn copy_path_local(src: &Path, dst: &Path) -> Result<()> {
  let meta = fs::symlink_metadata(src)?;
  let file_type = meta.file_type();

  if file_type.is_symlink() {
    #[cfg(unix)]
    {
      std::os::unix::fs::symlink(fs::read_link(src)?, dst)?;
      return Ok(());
    }

    #[cfg(not(unix))]
    bail!("copying symlinks is only supported on Unix");
  }

  if file_type.is_dir() {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
      let entry = entry?;
      copy_path_local(&entry.path(), &dst.join(entry.file_name()))?;
    }
    return Ok(());
  }

  if file_type.is_file() {
    fs::copy(src, dst)?;
    return Ok(());
  }

  bail!("unsupported source file type: {}", src.display())
}

fn run_doas<const N: usize>(args: [&str; N], path: Option<&Path>) -> Result<()> {
  let mut command = Command::new("doas");
  command.args(args);
  if let Some(path) = path {
    command.arg(path);
  }

  let status = command.status().context("failed to execute doas")?;
  if !status.success() {
    bail!("doas command failed with status {status}");
  }

  Ok(())
}

#[cfg(unix)]
fn requires_privilege_for_path(path: &Path) -> bool {
  let mut cursor = path.parent().unwrap_or_else(|| Path::new("/")).to_path_buf();

  while !cursor.exists() {
    let Some(parent) = cursor.parent() else {
      return true;
    };
    cursor = parent.to_path_buf();
  }

  let Ok(meta) = fs::metadata(&cursor) else {
    return true;
  };

  let mode = meta.mode();
  let owner_uid = meta.uid();
  let owner_gid = meta.gid();
  let current_uid = unsafe { libc::geteuid() } as u32;
  let current_gid = unsafe { libc::getegid() } as u32;

  let writable = if owner_uid == current_uid {
    mode & 0o200 != 0
  } else if owner_gid == current_gid {
    mode & 0o020 != 0
  } else {
    mode & 0o002 != 0
  };

  !writable
}

#[cfg(not(unix))]
fn requires_privilege_for_path(_path: &Path) -> bool {
  false
}

#[cfg(test)]
mod tests {
  use super::normalize_path;
  use std::path::Path;

  #[test]
  fn normalize_removes_dot_and_parent_components() {
    assert_eq!(
      normalize_path(Path::new("/tmp/a/./b/../c")),
      Path::new("/tmp/a/c")
    );
  }

  #[test]
  fn normalize_preserves_leading_parent_for_relative_paths() {
    assert_eq!(normalize_path(Path::new("../a/../b")), Path::new("../b"));
  }
}
