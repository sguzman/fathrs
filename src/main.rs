mod cli;
mod link;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{
  Context,
  Result,
  anyhow
};
use cli::{
  Args,
  LINKS_SCHEMA_PATH,
  expand_home_path
};
use link::link_one;
use serde::Deserialize;
use tracing::{
  error,
  info,
  instrument
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
struct LinksToml(
  BTreeMap<
    String,
    BTreeMap<String, String>
  >
);

fn main() -> Result<()> {
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
  let config_candidate =
    expand_home_path(&args.config);
  let config_path =
    fs::canonicalize(&config_candidate)
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

  let base_dir = args
    .base_dir
    .map(|bd| expand_home_path(&bd))
    .unwrap_or_else(|| {
      config_dir.clone()
    });
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
