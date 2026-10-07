use std::collections::BTreeSet;
use std::env;
use std::ffi::{CString, OsString};
use std::fs;
use std::io::{self, BufReader, Read};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
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
    bail!("post-change verification failed for {}: {}", entry.dst.display(), final_state.label());
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

  match fs::symlink_metadata(&entry.dst) {
    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
    Ok(_) => {
      bail!("post-unlink verification failed; destination still exists: {}", entry.dst.display())
    }
    Err(error) => {
      Err(error).with_context(|| format!("failed to verify removal of {}", entry.dst.display()))
    }
  }
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
      Component::Prefix(_) | Component::RootDir => out.push(component.as_os_str()),
      Component::CurDir => {}
      Component::ParentDir => {
        if matches!(out.components().next_back(), Some(Component::Normal(_))) {
          out.pop();
        } else if !out.has_root() {
          out.push("..");
        }
      }
      Component::Normal(value) => out.push(value),
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

#[derive(Clone, Copy, Debug)]
enum DoasOperation<'a> {
  Mkdir(&'a Path),
  RemoveFile(&'a Path),
  RemoveDir(&'a Path),
  Symlink { target: &'a Path, link: &'a Path },
  Copy { src: &'a Path, dst: &'a Path },
}

fn doas_args(operation: DoasOperation<'_>) -> Vec<OsString> {
  match operation {
    DoasOperation::Mkdir(path) => vec!["mkdir".into(), "-p".into(), "--".into(), path.into()],
    DoasOperation::RemoveFile(path) => {
      vec!["rm".into(), "-f".into(), "--".into(), path.into()]
    }
    DoasOperation::RemoveDir(path) => {
      vec!["rm".into(), "-rf".into(), "--".into(), path.into()]
    }
    DoasOperation::Symlink { target, link } => {
      vec!["ln".into(), "-s".into(), "--".into(), target.into(), link.into()]
    }
    DoasOperation::Copy { src, dst } => {
      vec!["cp".into(), "-a".into(), "--".into(), src.into(), dst.into()]
    }
  }
}

fn run_doas(operation: DoasOperation<'_>) -> Result<()> {
  let args = doas_args(operation);
  let status = Command::new("doas")
    .args(&args)
    .status()
    .with_context(|| format!("failed to execute doas with arguments {args:?}"))?;

  if !status.success() {
    bail!("doas command failed with status {status}: {args:?}");
  }

  Ok(())
}

fn ensure_dir_all(path: &Path, use_doas: bool) -> Result<()> {
  if path.exists() {
    return Ok(());
  }

  if use_doas {
    run_doas(DoasOperation::Mkdir(path))?;
  } else {
    fs::create_dir_all(path)?;
  }

  Ok(())
}

fn remove_any_path(path: &Path, use_doas: bool) -> Result<()> {
  let meta = fs::symlink_metadata(path)?;
  let file_type = meta.file_type();

  if use_doas {
    let operation = if file_type.is_dir() && !file_type.is_symlink() {
      DoasOperation::RemoveDir(path)
    } else {
      DoasOperation::RemoveFile(path)
    };
    run_doas(operation)?;
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
    run_doas(DoasOperation::Symlink { target, link: link_path })?;
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
    run_doas(DoasOperation::Copy { src, dst })?;
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

#[cfg(unix)]
fn requires_privilege_for_path(path: &Path) -> bool {
  let mut cursor = path.parent().unwrap_or_else(|| Path::new("/")).to_path_buf();

  while !cursor.exists() {
    let Some(parent) = cursor.parent() else {
      return true;
    };
    cursor = parent.to_path_buf();
  }

  let Ok(path_bytes) = CString::new(cursor.as_os_str().as_bytes()) else {
    return true;
  };

  let mode = libc::W_OK | libc::X_OK;
  unsafe { libc::access(path_bytes.as_ptr(), mode) != 0 }
}

#[cfg(not(unix))]
fn requires_privilege_for_path(_path: &Path) -> bool {
  false
}

#[cfg(test)]
mod tests {
  use super::{DoasOperation, doas_args, normalize_path};
  use std::ffi::OsString;
  use std::path::Path;

  #[test]
  fn normalize_removes_dot_and_parent_components() {
    assert_eq!(normalize_path(Path::new("/tmp/a/./b/../c")), Path::new("/tmp/a/c"));
  }

  #[test]
  fn normalize_preserves_leading_parent_for_relative_paths() {
    assert_eq!(normalize_path(Path::new("../a/../b")), Path::new("../b"));
  }

  #[test]
  fn normalize_preserves_multiple_leading_parents() {
    assert_eq!(normalize_path(Path::new("../../a")), Path::new("../../a"));
  }

  #[test]
  fn normalize_does_not_escape_filesystem_root() {
    assert_eq!(normalize_path(Path::new("/../../a")), Path::new("/a"));
  }

  #[test]
  fn privileged_copy_command_is_doas_cp_archive() {
    assert_eq!(
      doas_args(DoasOperation::Copy { src: Path::new("/src"), dst: Path::new("/dst") }),
      vec![
        OsString::from("cp"),
        OsString::from("-a"),
        OsString::from("--"),
        OsString::from("/src"),
        OsString::from("/dst"),
      ]
    );
  }

  #[test]
  fn privileged_directory_removal_is_explicitly_recursive() {
    assert_eq!(
      doas_args(DoasOperation::RemoveDir(Path::new("/target"))),
      vec![
        OsString::from("rm"),
        OsString::from("-rf"),
        OsString::from("--"),
        OsString::from("/target"),
      ]
    );
  }

  #[test]
  fn privileged_symlink_command_keeps_target_and_link_order() {
    assert_eq!(
      doas_args(DoasOperation::Symlink {
        target: Path::new("/source"),
        link: Path::new("/target"),
      }),
      vec![
        OsString::from("ln"),
        OsString::from("-s"),
        OsString::from("--"),
        OsString::from("/source"),
        OsString::from("/target"),
      ]
    );
  }
}
