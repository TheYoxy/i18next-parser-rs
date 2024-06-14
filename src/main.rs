#![cfg_attr(debug_assertions, allow(dead_code))]
use clap::Parser;

use std::collections::HashMap;

use color_eyre::eyre::Result;
use log::{debug, info};

use helper::MergeResult;

use crate::file::parse_directory;
use crate::file::write_to_file;
use crate::{
  cli::Cli,
  config::Config,
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
mod tests;
mod transform;
mod utils;
mod visitor;

fn main() -> Result<()> {
  initialize_panic_handler()?;
  initialize_logging()?;

  let cli = Cli::parse();
  let path = &cli.path;
  printinfo!("Looking for translations in path {path:?}");

  info!("Working directory: {:?}", path);
  let config = &Config::new(path, cli.verbose)?;
  debug!("Configuration: {config:?}");

  debug!("Actual configuration: {config:?}");
  let entries = parse_directory(path, config)?;
  write_to_file(entries, config)?;

  Ok(())
}

fn print_counts(
  locale: &str,
  namespace: &str,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  merged: &MergeResult,
  old_merged: &MergeResult,
  config: &Config,
) {
  let merge_count = merged.merge_count;
  let restore_count = old_merged.merge_count;
  let old_count = merged.old_count;
  let reset_count = merged.reset_count;
  println!("[{}] {}", locale, namespace);
  let unique_count = unique_count.get(namespace).unwrap_or(&0);
  let unique_plurals_count = unique_plurals_count.get(namespace).unwrap_or(&0);
  println!("Unique keys: {} ({} are plurals)", unique_count, unique_plurals_count);
  let add_count = unique_count.saturating_sub(merge_count);
  println!("Added keys: {}", add_count);
  println!("Restored keys: {}", restore_count);
  if config.keep_removed {
    println!("Unreferenced keys: {}", old_count);
  } else {
    println!("Removed keys: {}", old_count);
  }
  if config.reset_default_value_locale.is_some() {
    println!("Reset keys: {}", reset_count);
  }
  println!();
}
