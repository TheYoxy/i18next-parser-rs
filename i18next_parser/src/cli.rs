//! This module provides the CLI for the i18n system.
use std::{collections::HashMap, path::PathBuf};

use anstyle::Style;
use clap::{builder::Styles, command, Parser};
use clap_complete::Shell;
use color_eyre::owo_colors::OwoColorize;
use i18next_parser_core::{
  generate_index,
  generate_types,
  log_time,
  merge_all_values,
  parse_directory,
  print_config,
  write_to_file,
  Config,
};
use log::{info, trace};
use resolve_path::PathResolveExt;

use crate::print_count::{CountResults, PrintCounts};

/// Create the style used by the CLI
fn make_style() -> Styles {
  Styles::plain()
    .header(Style::new().bold())
    .literal(Style::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))))
}

/// The CLI options
#[derive(Parser, Debug)]
#[command(version, about, author, long_about= None, styles=make_style())]
pub struct Cli {
  /// The path to extract the translations from
  #[arg(value_name = "PATH", default_value = ".", global = true, value_hint = clap::ValueHint::DirPath)]
  path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false", global = true)]
  pub verbose: bool,
  /// Should generate types
  #[arg(short, long, default_value = "false", global = true)]
  #[cfg(feature = "generate_types")]
  generate_types: bool,

  /// Dry run (not writing to file)
  #[arg(short = 'n', long, default_value = "false", global = true)]
  dry_run: bool,

  /// Should generate shell completions
  #[arg(long)]
  #[clap(value_enum)]
  generate_shell: Option<Shell>,
}

impl Cli {
  /// Get if the shell should be generated
  pub fn generate_shell(&self) -> Option<Shell> {
    self.generate_shell
  }
}

pub trait Runnable {
  fn run(&self) -> color_eyre::Result<()>;
}

impl Runnable for Cli {
  fn run(&self) -> color_eyre::Result<()> {
    let path = &self.path;
    info!("Working directory: {}", path.display().yellow());
    let config = &Config::new(path, self.verbose, self.dry_run)?;
    trace!("Configuration: {config:?}");

    print_config(config);

    let path = &path.resolve();
    let entries = parse_directory(path.clone(), config)?;
    let merged = merge_all_values(entries, config)?;

    if config.verbose {
      merged
        .iter()
        .fold(CountResults::new(), |mut curr, result| {
          let mut map = HashMap::new();
          map.insert(&result.locale, &result.merged);
          if curr.contains_key(&result.namespace) {
            curr.get_mut(&result.namespace).and_then(|map| map.insert(&result.locale, &result.merged));
          } else {
            curr.insert(&result.namespace, map);
          }
          curr
        })
        .print_counts();
    }

    if config.dry_run {
      log::warn!("Dry run, not writing to file");
    } else {
      write_to_file(&merged, config)?;
    }

    if cfg!(feature = "generate_types") && self.generate_types {
      log_time!("Generating types", { generate_types(&merged, config) })?;
      log_time!("Generating types", { generate_index(&merged, config) })
    } else {
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test_log::test]
  fn should_parse_cli() {
    let cli = Cli::parse();
    assert_eq!(cli.path, PathBuf::from("."));
    assert!(!cli.verbose);
  }
}
