#![cfg_attr(debug_assertions, allow(dead_code))]

use std::time::Instant;

use clap::Parser;
use color_eyre::eyre::Result;
use log::{debug, info};

use crate::{
  cli::Cli,
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

  let entries = parse_directory(path, config)?;
  let merged = merge_all_values(entries, config)?;
  write_to_file(&merged, config)?;

  let elapsed = now.elapsed();
  let path = path.file_name().unwrap();
  printinfo!("Directory {path:?} parsed in {:.2}ms", elapsed.as_millis());
  if cli.generate_types {
    generate_types::generate_types(&merged, config)?;
  }

  Ok(())
}
