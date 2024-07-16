//! # i18next_parser
//! A rust rewrite of the [`i18next-parser`](https://github.com/i18next/i18next-parser) written in plain js

use clap::Parser;
use color_eyre::eyre::Result;

use crate::{
  cli::{Cli, Runnable},
  completion::generate_completion,
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

mod completion {
  //! This module provides functionality for generating shell completions
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
  if let Some(shell) = cli.generate_shell() {
    generate_completion(shell)
  } else {
    print_app();
    initialize_panic_handler()?;
    initialize_logging()?;

    let cli = Cli::parse();
    cli.run()
  }
}
