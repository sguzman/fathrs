use std::path::{
  Path,
  PathBuf
};
use std::process::Command;

fn repo_root() -> PathBuf {
  // Cargo sets this at compile time for
  // tests.
  PathBuf::from(env!(
    "CARGO_MANIFEST_DIR"
  ))
}

fn example_dir() -> PathBuf {
  repo_root()
    .join("examples")
    .join("test1")
}

#[cfg(unix)]
fn assert_symlink_points_to(
  link: &Path,
  expected_target: &Path
) {
  assert!(
    expected_target.exists(),
    "expected target does not exist: \
     {}",
    expected_target.display()
  );

  let actual = std::fs::read_link(link)
    .unwrap_or_else(|e| {
      panic!(
        "read_link failed for {}: {e}",
        link.display()
      )
    });

  let actual_norm = if actual
    .is_absolute()
  {
    actual
  } else {
    link.parent().unwrap().join(actual)
  };

  let actual_can =
    std::fs::canonicalize(&actual_norm)
      .unwrap_or_else(|e| {
        panic!(
          "canonicalize(actual) \
           failed for {}: {e}",
          actual_norm.display()
        )
      });

  let expected_can =
    std::fs::canonicalize(
      expected_target
    )
    .unwrap_or_else(|e| {
      panic!(
        "canonicalize(expected) \
         failed for {}: {e}",
        expected_target.display()
      )
    });

  assert_eq!(
    actual_can,
    expected_can,
    "symlink {} points to {}, \
     expected {}",
    link.display(),
    actual_can.display(),
    expected_can.display(),
  );
}

#[test]
fn links_toml_example_test1_creates_expected_symlinks()
 {
  // Arrange
  let ex = example_dir();

  let links = ex.join("links.toml");
  assert!(
    links.exists(),
    "missing links.toml at {}",
    links.display()
  );

  let local_source =
    ex.join("local-source");
  let local_target =
    ex.join("local-target");

  // Clean target dir so the test is
  // repeatable. (We only remove the
  // example's target directory.)
  let _ = std::fs::remove_dir_all(
    &local_target
  );
  std::fs::create_dir_all(
    &local_target
  )
  .unwrap();

  // Act: run the built binary via
  // Cargo's integration test helper env
  // var. Cargo sets
  // CARGO_BIN_EXE_<name> for each
  // binary target.
  let exe =
    env!("CARGO_BIN_EXE_fathrs");

  let output = Command::new(exe)
        .current_dir(&repo_root()) // IMPORTANT: run from repo root like you did
        .env("RUST_LOG", "trace")
        .arg("--config")
        .arg(&links)
        .arg("--base-dir")
        .arg(&ex)
        .arg("--force")
        .output()
        .expect("failed to run fathrs");

  if !output.status.success() {
    panic!(
      "fathrs failed.\nstatus: \
       {}\nstdout:\n{}\nstderr:\n{}",
      output.status,
      String::from_utf8_lossy(
        &output.stdout
      ),
      String::from_utf8_lossy(
        &output.stderr
      ),
    );
  }

  // Assert: symlinks exist and point to
  // the right sources
  let link1 =
    local_target.join("test1.txt");
  let link2 =
    local_target.join("test2.txt");
  let link_dir =
    local_target.join("local-dir");

  assert!(
    link1.exists(),
    "expected {}",
    link1.display()
  );
  assert!(
    link2.exists(),
    "expected {}",
    link2.display()
  );
  assert!(
    link_dir.exists(),
    "expected {}",
    link_dir.display()
  );

  // Verify they are symlinks (and not
  // copied files/dirs)
  let md1 =
    std::fs::symlink_metadata(&link1)
      .unwrap();
  let md2 =
    std::fs::symlink_metadata(&link2)
      .unwrap();
  let mdd = std::fs::symlink_metadata(
    &link_dir
  )
  .unwrap();

  assert!(
    md1.file_type().is_symlink(),
    "{} is not a symlink",
    link1.display()
  );
  assert!(
    md2.file_type().is_symlink(),
    "{} is not a symlink",
    link2.display()
  );
  assert!(
    mdd.file_type().is_symlink(),
    "{} is not a symlink",
    link_dir.display()
  );

  // Verify targets
  assert_symlink_points_to(
    &link1,
    &local_source.join("test1.txt")
  );
  assert_symlink_points_to(
    &link2,
    &local_source.join("test2.txt")
  );
  assert_symlink_points_to(
    &link_dir,
    &local_source.join("test-dir")
  );

  // Bonus: ensure the dir link contains
  // the expected file via traversal
  let linked_file =
    link_dir.join("test3.txt");
  assert!(
    linked_file.exists(),
    "expected {}",
    linked_file.display()
  );
}
