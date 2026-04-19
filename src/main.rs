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
use clap::Parser;
use cli::{
  Args,
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
struct LinksToml(BTreeMap<String, Section>);

#[derive(Debug, Deserialize)]
struct Section {
  #[serde(default)]
  sudo: bool,
  #[serde(flatten)]
  links: BTreeMap<String, LinkValue>
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LinkValue {
  Simple(String),
  Detailed {
    target: String,
    sudo: Option<bool>
  }
}

impl LinkValue {
  fn target(&self) -> &str {
    match self {
      | LinkValue::Simple(s) => s,
      | LinkValue::Detailed {
        target, ..
      } => target
    }
  }

  fn sudo(&self, section_sudo: bool) -> bool {
    match self {
      | LinkValue::Simple(_) =>
        section_sudo,
      | LinkValue::Detailed {
        sudo, ..
      } => sudo.unwrap_or(section_sudo)
    }
  }
}

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

#[instrument(level = "trace", skip_all, fields(config=?args.config))]
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
    .and_then(|candidate| {
      if candidate.is_absolute() {
        None
      } else {
        Some(config_dir.join(candidate))
      }
    })
    .unwrap_or_else(|| {
      config_dir.clone()
    });

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

  let command =
    args.command.unwrap_or(
      cli::Command::Link {
        force: false,
        dry_run: false
      }
    );

  match command {
    | cli::Command::Validate => {
      info!(?config_path, "validation successful");
      return Ok(());
    }
    | cli::Command::Link {
      force,
      dry_run
    } => {
      info!(
        ?config_path,
        ?base_dir,
        force,
        dry_run,
        "starting link"
      );
      process_links(
        &links, &base_dir, force,
        dry_run, false, false
      )?;
    }
    | cli::Command::Probe {
      warn_only
    } => {
      info!(
        ?config_path,
        ?base_dir,
        warn_only,
        "starting probe"
      );
      process_links(
        &links, &base_dir, false,
        false, true, warn_only
      )?;
    }
  }

  Ok(())
}

fn process_links(
  links: &LinksToml,
  base_dir: &Path,
  force: bool,
  dry_run: bool,
  status: bool,
  warn_only: bool
) -> Result<()> {
  let mut total = 0usize;
  for (section_name, section) in &links.0 {
    info!(
      section = %section_name,
      count = section.links.len(),
      sudo = section.sudo,
      "processing section"
    );
    for (src_str, val) in &section.links {
      total += 1;
      let dst_str = val.target();
      let use_sudo =
        val.sudo(section.sudo);

      link_one(
        base_dir,
        Path::new(src_str),
        Path::new(dst_str),
        force,
        dry_run,
        status,
        warn_only,
        section_name,
        use_sudo
      )
      .with_context(|| {
        format!(
          "section={section_name} \
           src={src_str} dst={dst_str}"
        )
      })?;
    }
  }
  info!(total, "done");
  Ok(())
}
