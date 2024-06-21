#![cfg_attr(debug_assertions, allow(dead_code))]

use std::time::Instant;

use clap::Parser;
use color_eyre::eyre::Result;
use log::{debug, info};

use crate::print::print_app::print_app;
use crate::print::print_config::print_config;
use crate::{
  cli::Cli,
  config::Config,
  file::parse_directory,
  file::write_to_file,
  utils::{initialize_logging, initialize_panic_handler},
};

mod catalog;
mod cli;
mod config;
mod file;
mod helper;
mod is_empty;
mod macros;
mod plural;
mod print;
mod tests;
mod transform;
mod utils;
mod visitor;

fn main() -> Result<()> {
  print_app();
  initialize_panic_handler()?;
  initialize_logging()?;

  let cli = Cli::parse();
  let path = &cli.path;

  info!("Working directory: {}", path.display());
  let config = &Config::new(path, cli.verbose)?;
  debug!("Configuration: {config:?}");

  print_config(config);

  let now = Instant::now();

  parse_directory(path, config).and_then(|entries| write_to_file(entries, config))?;

  let elapsed = now.elapsed();
  printinfo!("Directory {path:?} parsed in {:.2}ms", elapsed.as_millis());

  Ok(())
}
