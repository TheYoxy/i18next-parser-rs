#![cfg_attr(debug_assertions, allow(dead_code))]
use clap::Parser;

use std::collections::HashMap;

use color_eyre::{eyre::Result, owo_colors::OwoColorize};
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

fn print_app() {
  let name = env!("CARGO_CRATE_NAME");
  let version = env!("CARGO_PKG_VERSION");
  println!("{name} {version}");
}

fn print_config(config: &crate::config::Config) {
  println!("  {}", "i18next Parser rust".bright_cyan());
  println!("  {}", "--------------".bright_cyan());
  let input = config.input.iter().map(|input| input.as_str()).collect::<Vec<_>>().join(", ");
  println!("  {} {}", "Input: ".bright_cyan(), input);

  println!("  {} {}", "Output:".bright_cyan(), config.output);
}

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

  let entries = parse_directory(path, config);
  let entries = match entries {
    Ok(entries) => entries,
    Err(e) => {
      printerror!("Error parsing directory: {e}");
      return Err(e);
    },
  };

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
