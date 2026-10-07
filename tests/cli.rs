use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
  root: PathBuf,
  home: PathBuf,
}

impl Fixture {
  fn new(name: &str) -> Self {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
      "fathrs-test-{name}-{}-{id}",
      std::process::id()
    ));
    let home = root.join("home");

    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&home).unwrap();

    Self { root, home }
  }

  fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
    self.root.join(relative)
  }

  fn write(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
    let path = self.path(relative);
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, contents).unwrap();
    path
  }

  fn config(&self, contents: &str) -> PathBuf {
    self.write("links.toml", contents)
  }

  fn run_paths(&self, args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fathrs"))
      .current_dir(&self.root)
      .env("HOME", &self.home)
      .args(args)
      .output()
      .unwrap()
  }
}

impl Drop for Fixture {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.root);
  }
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}

#[cfg(unix)]
fn assert_symlink_to(link: &Path, target: &Path) {
  let link_target = fs::read_link(link).unwrap();
  let resolved = if link_target.is_absolute() {
    link_target
  } else {
    link.parent().unwrap().join(link_target)
  };
  assert_eq!(fs::canonicalize(resolved).unwrap(), fs::canonicalize(target).unwrap());
}

#[test]
fn validate_accepts_doas_and_legacy_sudo_alias() {
  let fixture = Fixture::new("validate-alias");
  fixture.write("a", "a");
  fixture.write("b", "b");
  let config = fixture.config(
    r#"
[modern]
doas = false
"a" = "out-a"

[legacy]
sudo = false
"b" = "out-b"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "validate".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
}

#[test]
fn links_a_file_and_is_idempotent() {
  let fixture = Fixture::new("file-link");
  let source = fixture.write("source.txt", "hello");
  let destination = fixture.path("target.txt");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let first = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(first.status.success(), "stderr: {}", stderr(&first));
  assert_symlink_to(&destination, &source);

  let second = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(second.status.success(), "stderr: {}", stderr(&second));
  assert_symlink_to(&destination, &source);
}

#[test]
fn links_a_directory() {
  let fixture = Fixture::new("dir-link");
  fixture.write("source-dir/file.txt", "hello");
  let source = fixture.path("source-dir");
  let destination = fixture.path("target-dir");
  let config = fixture.config(
    r#"
[dotfiles]
"source-dir" = "target-dir"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &source);
  assert_eq!(fs::read_to_string(destination.join("file.txt")).unwrap(), "hello");
}

#[test]
fn absolute_base_dir_is_used_as_supplied() {
  let fixture = Fixture::new("absolute-base");
  let base = fixture.path("base");
  fs::create_dir_all(&base).unwrap();
  fs::write(base.join("source.txt"), "hello").unwrap();
  let destination = fixture.path("target.txt");
  let config = fixture.config(&format!(
    "[dotfiles]\n\"source.txt\" = \"{}\"\n",
    destination.display()
  ));

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "--base-dir".as_ref(),
    base.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &base.join("source.txt"));
}

#[test]
fn dry_run_leaves_filesystem_untouched() {
  let fixture = Fixture::new("dry-run");
  fixture.write("source.txt", "hello");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
    "--dry-run".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert!(!fixture.path("target.txt").exists());
}

#[test]
fn conflict_without_force_fails_without_replacing_destination() {
  let fixture = Fixture::new("no-force");
  fixture.write("source.txt", "new");
  fixture.write("target.txt", "old");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(!output.status.success());
  assert_eq!(fs::read_to_string(fixture.path("target.txt")).unwrap(), "old");
}

#[test]
fn force_replaces_conflicting_file() {
  let fixture = Fixture::new("force");
  let source = fixture.write("source.txt", "new");
  let destination = fixture.write("target.txt", "old");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
    "--force".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &source);
}

#[test]
fn missing_later_source_prevents_earlier_mutation() {
  let fixture = Fixture::new("preflight");
  fixture.write("present.txt", "hello");
  let config = fixture.config(
    r#"
[dotfiles]
"present.txt" = "target-present.txt"
"missing.txt" = "target-missing.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(!fixture.path("target-present.txt").exists());
}

#[test]
fn copy_mode_is_idempotent_and_probe_detects_drift() {
  let fixture = Fixture::new("copy-drift");
  fixture.write("source.txt", "hello");
  let destination = fixture.path("target.txt");
  let config = fixture.config(
    r#"
[dotfiles]
copy = true
"source.txt" = "target.txt"
"#,
  );

  let link = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(link.status.success(), "stderr: {}", stderr(&link));
  assert_eq!(fs::read_to_string(&destination).unwrap(), "hello");

  let second = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(second.status.success(), "stderr: {}", stderr(&second));

  fs::write(&destination, "drift").unwrap();
  let probe = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);

  assert!(!probe.status.success());
  assert!(stdout(&probe).contains("DRIFTED"));
}

#[test]
fn copy_mode_tracks_directory_drift() {
  let fixture = Fixture::new("copy-dir");
  fixture.write("source/a.txt", "a");
  fixture.write("source/nested/b.txt", "b");
  let destination = fixture.path("target");
  let config = fixture.config(
    r#"
[dotfiles]
"source" = { target = "target", copy = true }
"#,
  );

  let link = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(link.status.success(), "stderr: {}", stderr(&link));

  let probe_ok = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);
  assert!(probe_ok.status.success(), "stderr: {}", stderr(&probe_ok));

  fs::write(destination.join("nested/b.txt"), "changed").unwrap();
  let probe_bad = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);
  assert!(!probe_bad.status.success());
  assert!(stdout(&probe_bad).contains("DRIFTED"));
}

#[test]
fn per_entry_copy_override_beats_section_default() {
  let fixture = Fixture::new("copy-override");
  let linked_source = fixture.write("linked.txt", "linked");
  fixture.write("copied.txt", "copied");
  let config = fixture.config(
    r#"
[dotfiles]
copy = true
"copied.txt" = "copy-target.txt"
"linked.txt" = { target = "link-target.txt", copy = false }
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_eq!(
    fs::read_to_string(fixture.path("copy-target.txt")).unwrap(),
    "copied"
  );
  assert_symlink_to(&fixture.path("link-target.txt"), &linked_source);
}

#[test]
fn probe_reports_wrong_symlink_target() {
  let fixture = Fixture::new("wrong-target");
  fixture.write("source.txt", "source");
  fixture.write("other.txt", "other");
  #[cfg(unix)]
  std::os::unix::fs::symlink(
    fixture.path("other.txt"),
    fixture.path("target.txt"),
  )
  .unwrap();
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(stdout(&output).contains("WRONG-TARGET"));
}

#[test]
fn validate_rejects_duplicate_destinations() {
  let fixture = Fixture::new("duplicate-destination");
  let config = fixture.config(
    r#"
[a]
"one.txt" = "target.txt"

[b]
"two.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "validate".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("same destination"));
}

#[test]
fn validate_rejects_source_equal_to_destination() {
  let fixture = Fixture::new("same-path");
  let config = fixture.config(
    r#"
[dotfiles]
"same.txt" = "same.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "validate".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("same path"));
}

#[test]
fn warn_only_probe_suppresses_ok_entries() {
  let fixture = Fixture::new("warn-only");
  fixture.write("source.txt", "hello");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let link = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(link.status.success(), "stderr: {}", stderr(&link));

  let probe = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
    "--warn-only".as_ref(),
  ]);

  assert!(probe.status.success(), "stderr: {}", stderr(&probe));
  assert!(stdout(&probe).trim().is_empty());
}

#[test]
fn home_expansion_uses_home_environment() {
  let fixture = Fixture::new("home-expansion");
  fixture.write("source.txt", "hello");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "~/.config/fathrs-test.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(
    &fixture.home.join(".config/fathrs-test.txt"),
    &fixture.path("source.txt"),
  );
}

#[test]
fn unlink_removes_only_managed_symlink() {
  let fixture = Fixture::new("unlink");
  fixture.write("source.txt", "hello");
  let destination = fixture.path("target.txt");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let link = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(link.status.success(), "stderr: {}", stderr(&link));
  assert!(fs::symlink_metadata(&destination).is_ok());

  let dry = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "unlink".as_ref(),
    "--dry-run".as_ref(),
  ]);
  assert!(dry.status.success(), "stderr: {}", stderr(&dry));
  assert!(fs::symlink_metadata(&destination).is_ok());

  let unlink = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "unlink".as_ref(),
  ]);
  assert!(unlink.status.success(), "stderr: {}", stderr(&unlink));
  assert!(fs::symlink_metadata(&destination).is_err());
}

#[test]
fn unlink_refuses_drifted_copy() {
  let fixture = Fixture::new("unlink-drift");
  fixture.write("source.txt", "source");
  let destination = fixture.path("target.txt");
  let config = fixture.config(
    r#"
[dotfiles]
copy = true
"source.txt" = "target.txt"
"#,
  );

  let link = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(link.status.success(), "stderr: {}", stderr(&link));

  fs::write(&destination, "local change").unwrap();

  let unlink = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "unlink".as_ref(),
  ]);
  assert!(!unlink.status.success());
  assert_eq!(fs::read_to_string(&destination).unwrap(), "local change");
  assert!(stderr(&unlink).contains("refusing to unlink"));
}

#[test]
fn force_replaces_conflicting_directory_tree() {
  let fixture = Fixture::new("force-directory");
  let source = fixture.path("source-dir");
  fixture.write("source-dir/new.txt", "new");
  fixture.write("target-dir/old.txt", "old");
  let destination = fixture.path("target-dir");
  let config = fixture.config(
    r#"
[dotfiles]
"source-dir" = "target-dir"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
    "--force".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &source);
  assert_eq!(fs::read_to_string(destination.join("new.txt")).unwrap(), "new");
}

#[test]
fn force_replaces_wrong_symlink_target() {
  let fixture = Fixture::new("force-wrong-link");
  let source = fixture.write("source.txt", "source");
  fixture.write("other.txt", "other");
  let destination = fixture.path("target.txt");
  #[cfg(unix)]
  std::os::unix::fs::symlink(fixture.path("other.txt"), &destination).unwrap();
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
    "--force".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &source);
}

#[test]
fn probe_reports_broken_symlink_as_wrong_target() {
  let fixture = Fixture::new("broken-link");
  fixture.write("source.txt", "source");
  let destination = fixture.path("target.txt");
  #[cfg(unix)]
  std::os::unix::fs::symlink(fixture.path("does-not-exist"), &destination).unwrap();
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "target.txt"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(stdout(&output).contains("WRONG-TARGET"));
}

#[test]
fn copied_directory_preserves_symlink_entries() {
  let fixture = Fixture::new("copy-symlink");
  fixture.write("source/real.txt", "hello");
  #[cfg(unix)]
  std::os::unix::fs::symlink("real.txt", fixture.path("source/link.txt")).unwrap();
  let config = fixture.config(
    r#"
[dotfiles]
"source" = { target = "target", copy = true }
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "link".as_ref(),
  ]);
  assert!(output.status.success(), "stderr: {}", stderr(&output));

  let copied_link = fixture.path("target/link.txt");
  assert!(fs::symlink_metadata(&copied_link).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_link(copied_link).unwrap(), Path::new("real.txt"));

  let probe = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "probe".as_ref(),
  ]);
  assert!(probe.status.success(), "stderr: {}", stderr(&probe));
}

#[test]
fn validate_rejects_home_directory_as_destination() {
  let fixture = Fixture::new("dangerous-home");
  let config = fixture.config(
    r#"
[dotfiles]
"source.txt" = "~"
"#,
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "validate".as_ref(),
  ]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("dangerous destination"));
}

#[test]
fn relative_base_dir_resolves_from_config_directory() {
  let fixture = Fixture::new("relative-base");
  fixture.write("config/sources/source.txt", "hello");
  let destination = fixture.path("target.txt");
  let config = fixture.write(
    "config/links.toml",
    &format!(
      "[dotfiles]\n\"source.txt\" = \"{}\"\n",
      destination.display()
    ),
  );

  let output = fixture.run_paths(&[
    "--config".as_ref(),
    config.as_os_str(),
    "--base-dir".as_ref(),
    "sources".as_ref(),
    "link".as_ref(),
  ]);

  assert!(output.status.success(), "stderr: {}", stderr(&output));
  assert_symlink_to(&destination, &fixture.path("config/sources/source.txt"));
}
