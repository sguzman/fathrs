mod cli;
mod link;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use cli::{expand_home_path, Args};
use link::{apply_entry, probe_entry, unlink_entry, PlanEntry, ProbeState};
use serde::Deserialize;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
struct LinksToml(BTreeMap<String, Section>);

#[derive(Debug, Deserialize)]
struct Section {
  #[serde(default)]
  copy: bool,
  #[serde(default, alias = "sudo")]
  doas: bool,
  #[serde(flatten)]
  links: BTreeMap<String, LinkValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LinkValue {
  Simple(String),
  Detailed {
    target: String,
    copy: Option<bool>,
    #[serde(alias = "sudo")]
    doas: Option<bool>,
  },
}

impl LinkValue {
  fn target(&self) -> &str {
    match self {
      LinkValue::Simple(target) => target,
      LinkValue::Detailed { target, .. } => target,
    }
  }

  fn copy(&self, section_copy: bool) -> bool {
    match self {
      LinkValue::Simple(_) => section_copy,
      LinkValue::Detailed { copy, .. } => copy.unwrap_or(section_copy),
    }
  }

  fn doas(&self, section_doas: bool) -> bool {
    match self {
      LinkValue::Simple(_) => section_doas,
      LinkValue::Detailed { doas, .. } => doas.unwrap_or(section_doas),
    }
  }
}

fn main() -> Result<()> {
  tracing_subscriber::fmt()
    .with_env_filter(
      EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,fathrs=trace")),
    )
    .with_target(true)
    .with_level(true)
    .init();

  if let Err(error) = run(Args::parse()) {
    error!("{error:#}");
    std::process::exit(1);
  }

  Ok(())
}

fn run(args: Args) -> Result<()> {
  let config_candidate = expand_home_path(&args.config);
  let config_path = fs::canonicalize(&config_candidate)
    .with_context(|| format!("failed to resolve config path: {}", config_candidate.display()))?;
  let config_dir = config_path
    .parent()
    .ok_or_else(|| anyhow!("config file has no parent directory: {}", config_path.display()))?
    .to_path_buf();

  let base_dir = resolve_base_dir(args.base_dir.as_deref(), &config_dir);
  let raw = fs::read_to_string(&config_path)
    .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
  let links: LinksToml = toml::from_str(&raw)
    .with_context(|| format!("failed to parse TOML in {}", config_path.display()))?;

  let command = args.command.unwrap_or(cli::Command::Link {
    force: false,
    dry_run: false,
  });

  match command {
    cli::Command::Unlink { dry_run } => {
      let plan = build_plan(&links, &base_dir, true)?;
      info!(
        config = %config_path.display(),
        entries = plan.len(),
        dry_run,
        "unlink plan validated"
      );

      for entry in &plan {
        unlink_entry(entry, dry_run)?;
      }

      info!(entries = plan.len(), "unlink complete");
    }
    cli::Command::Validate => {
      let plan = build_plan(&links, &base_dir, false)?;
      info!(entries = plan.len(), "configuration is valid");
    }
    cli::Command::Link { force, dry_run } => {
      let plan = build_plan(&links, &base_dir, true)?;
      info!(
        config = %config_path.display(),
        base_dir = %base_dir.display(),
        entries = plan.len(),
        force,
        dry_run,
        "plan validated"
      );

      for entry in &plan {
        apply_entry(entry, force, dry_run)?;
      }

      info!(entries = plan.len(), "done");
    }
    cli::Command::Probe { warn_only } => {
      let plan = build_plan(&links, &base_dir, true)?;
      let mut drift = 0usize;

      for entry in &plan {
        let state = probe_entry(entry)?;
        if state != ProbeState::Ok {
          drift += 1;
        }

        if !warn_only || state != ProbeState::Ok {
          println!(
            "{} [{}] {} <- {}",
            state.label(),
            entry.section,
            entry.dst.display(),
            entry.src.display()
          );
        }

        if state != ProbeState::Ok && entry.write_requires_privilege() && !entry.use_doas {
          warn!(
            section = %entry.section,
            dst = %entry.dst.display(),
            "destination may require elevated write access but doas is disabled"
          );
        }
      }

      if drift > 0 {
        bail!("probe detected {drift} out-of-sync entr{}", if drift == 1 { "y" } else { "ies" });
      }

      info!(entries = plan.len(), "all configured destinations are in sync");
    }
  }

  Ok(())
}

fn resolve_base_dir(requested: Option<&Path>, config_dir: &Path) -> PathBuf {
  match requested {
    Some(path) => {
      let expanded = expand_home_path(path);
      if expanded.is_absolute() {
        link::normalize_path(&expanded)
      } else {
        link::normalize_path(&config_dir.join(expanded))
      }
    }
    None => config_dir.to_path_buf(),
  }
}

fn build_plan(links: &LinksToml, base_dir: &Path, check_sources: bool) -> Result<Vec<PlanEntry>> {
  let mut plan = Vec::new();
  let mut destinations = BTreeSet::new();

  for (section_name, section) in &links.0 {
    if section.links.is_empty() {
      bail!("section {section_name:?} contains no link entries");
    }

    for (src, value) in &section.links {
      if src.trim().is_empty() {
        bail!("section {section_name:?} contains an empty source path");
      }
      if value.target().trim().is_empty() {
        bail!("section {section_name:?} source {src:?} has an empty destination path");
      }

      let entry = PlanEntry::new(
        base_dir,
        section_name,
        Path::new(src),
        Path::new(value.target()),
        value.doas(section.doas),
        value.copy(section.copy),
      )?;
      entry.validate_static()?;

      if check_sources {
        entry.validate_source()?;
      }

      if !destinations.insert(entry.dst.clone()) {
        bail!(
          "multiple entries resolve to the same destination: {}",
          entry.dst.display()
        );
      }

      plan.push(entry);
    }
  }

  if plan.is_empty() {
    bail!("configuration contains no link entries");
  }

  Ok(plan)
}
