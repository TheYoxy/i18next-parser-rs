//! # i18next_parser
//! A rust rewrite of the [`i18next-parser`](https://github.com/i18next/i18next-parser) written in plain js

use std::time::Instant;

use clap::Parser;
use color_eyre::eyre::Result;
use log::{debug, info};

use crate::{
  cli::Cli,
  completion::generate_completion,
  config::Config,
  file::write_to_file,
  merger::merge_all_values::merge_all_values,
  parser::parse_directory::parse_directory,
  print::{print_app::print_app, print_config::print_config},
  utils::{initialize_logging, initialize_panic_handler},
};

mod catalog;
mod cli;
mod config;
mod file;
#[cfg(feature = "generate_types")]
mod generate_types;
mod helper;
mod is_empty;
mod macros;
mod merger;
mod parser;
mod plural;
mod print;
mod tests;
mod transform;
mod utils;
mod visitor;

pub(crate) mod completion {
  use clap::CommandFactory;
  use clap_complete::Shell;
  use log::debug;

  use crate::cli::Cli;

  fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
    use clap_complete::generate;
    debug!("Generating completions for command: {:?}", cmd.get_name());
    #[cfg(not(test))]
    let mut buf = std::io::stdout();
    #[cfg(test)]
    let mut buf = std::io::sink();
    generate(gen, cmd, cmd.get_name().to_string(), &mut buf);
  }

  pub(crate) fn generate_completion(shell: Shell) -> color_eyre::Result<()> {
    let mut cmd = Cli::command();
    debug!("Generating completions for shell: {}", shell);
    print_completions(shell, &mut cmd);
    Ok(())
  }
}

/// Entry point of the application
fn main() -> Result<()> {
  let cli = Cli::parse();
  if let Some(shell) = cli.generate_shell {
    return generate_completion(shell);
  }

  print_app();
  initialize_panic_handler()?;
  initialize_logging()?;

  let path = &cli.path;

  info!("Working directory: {}", path.display());
  let config = &Config::new(path, cli.verbose)?;
  debug!("Configuration: {config:?}");

  print_config(config);

  let now = Instant::now();

  let entries = parse_directory(path, config)?;
  let merged = merge_all_values(entries, config)?;
  write_to_file(&merged, config)?;

  let elapsed = now.elapsed();
  let path = path.file_name().unwrap();
  printinfo!("Directory {path:?} parsed in {:.2}ms", elapsed.as_millis());
  #[cfg(feature = "generate_types")]
  if cli.generate_types {
    generate_types::generate_types(&merged, config)?;
  }

  Ok(())
}
