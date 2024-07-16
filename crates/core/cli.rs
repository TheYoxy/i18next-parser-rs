use std::{env, path::PathBuf};

use anstyle::Style;
use clap::{builder::Styles, command, Parser};
use clap_complete::Shell;
use color_eyre::eyre::eyre;
use log::{info, trace};

use crate::{
  config::Config, file::write_to_file, generate_types, log_time, merger::merge_all_values::merge_all_values,
  parser::parse_directory::parse_directory, print::print_config::print_config,
};

/// Get the default log path
fn get_default_log_path() -> PathBuf {
  env::current_dir().unwrap()
}

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
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string(), global = true, value_hint = clap::ValueHint::DirPath)]
  path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false", global = true)]
  verbose: bool,
  /// Should generate types
  #[arg(short, long, default_value = "false", global = true)]
  #[cfg(feature = "generate_types")]
  generate_types: bool,

  /// Should generate shell completions
  #[arg(long)]
  #[clap(value_enum)]
  generate_shell: Option<Shell>,
}

impl Cli {
  pub(crate) fn generate_shell(&self) -> Option<Shell> {
    self.generate_shell
  }
}

pub trait Runnable {
  fn run(&self) -> color_eyre::Result<()>;
}

impl Runnable for Cli {
  fn run(&self) -> color_eyre::Result<()> {
    let path = &self.path;
    log_time!(format!("Parsing {} to find translations to extract", path.display().yellow()), {
      info!("Working directory: {}", path.display().yellow());
      let config = &Config::new(path, self.verbose)?;
      trace!("Configuration: {config:?}");

      print_config(config);

      let file_name = path.file_name().ok_or(eyre!("Invalid path"))?;
      let merged = log_time!(format!("Parsing directory {:?}", file_name.yellow()), {
        let entries = parse_directory(path, config)?;
        let merged = merge_all_values(entries, config)?;
        write_to_file(&merged, config)?;

        merged
      });
      #[cfg(feature = "generate_types")]
      if self.generate_types {
        log_time!("Generating types", { generate_types::generate_types(&merged, config) })
      } else {
        Ok(())
      }
      #[cfg(not(feature = "generate_types"))]
      Ok(())
    })
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
