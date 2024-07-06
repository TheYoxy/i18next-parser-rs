use std::{env, path::PathBuf};

use crate::config::Config;
use crate::file::write_to_file;
use crate::merger::merge_all_values::merge_all_values;
use crate::parser::parse_directory::parse_directory;
use crate::print::print_config::print_config;
use crate::{generate_types, log_time};
use clap::{command, Parser};
use color_eyre::eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use log::{debug, info, trace};

/// Get the default log path
fn get_default_log_path() -> PathBuf {
  env::current_dir().unwrap()
}

/// The CLI options
#[derive(Parser, Debug)]
#[command(version, about, author)]
pub struct Cli {
  /// The path to extract the translations from
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string())]
  path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false", global = true)]
  verbose: bool,
  /// Should generate types
  #[arg(short, long, default_value = "false", global = true)]
  #[cfg(feature = "generate_types")]
  generate_types: bool,
}

pub trait Runnable {
  fn run(&self) -> color_eyre::Result<()>;
}

impl Runnable for Cli {
  fn run(&self) -> color_eyre::Result<()> {
    let path = &self.path;

    info!("Working directory: {}", path.display().yellow());
    let config = &Config::new(path, self.verbose)?;
    trace!("Configuration: {config:?}");

    print_config(config);

    let file_name = path.file_name().ok_or(eyre!("Invalid path"))?;
    let merged = log_time!(format!("Parsing directory {file_name:?}"), {
      let entries = parse_directory(path, config)?;
      let merged = merge_all_values(entries, config)?;
      write_to_file(&merged, config)?;

      merged
    });
    #[cfg(feature = "generate_types")]
    if self.generate_types {
      generate_types::generate_types(&merged, config)
    } else {
      Ok(())
    }
    #[cfg(not(feature = "generate_types"))]
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test_log::test]
  fn should_parse_cli() {
    let cli = Cli::parse();
    assert_eq!(cli.path, get_default_log_path());
    assert!(!cli.verbose);
  }
}
