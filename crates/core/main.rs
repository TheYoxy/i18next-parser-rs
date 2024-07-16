//! # i18next_parser
//! A rust rewrite of the [`i18next-parser`](https://github.com/i18next/i18next-parser) written in plain js

use clap::Parser;
use color_eyre::eyre::Result;

use crate::{
  cli::{Cli, Runnable},
  print::print_app::print_app,
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

/// Entry point of the application
fn main() -> Result<()> {
  print_app();
  initialize_panic_handler()?;
  initialize_logging()?;

  let cli = Cli::parse();
  cli.run()
}
