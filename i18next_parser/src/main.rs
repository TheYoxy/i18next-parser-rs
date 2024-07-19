pub mod cli;
pub mod utils;

fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
  use clap_complete::generate;
  use log::debug;
  debug!("Generating completions for command: {:?}", cmd.get_name());
  if cfg!(test) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::sink())
  } else {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout())
  }
}

fn generate_completion(shell: clap_complete::Shell) -> color_eyre::Result<()> {
  use clap::CommandFactory;
  use log::debug;
  let mut cmd = cli::Cli::command();
  debug!("Generating completions for shell: {}", shell);
  print_completions(shell, &mut cmd);
  Ok(())
}

fn print_app() {
  let name = env!("CARGO_CRATE_NAME");
  let version = env!("CARGO_PKG_VERSION");
  eprintln!("{name} {version}");
}

/// Entry point of the application
fn main() -> color_eyre::Result<()> {
  use clap::Parser;

  use crate::{
    cli::{Cli, Runnable},
    utils::{initialize_logging, initialize_panic_handler},
  };
  let cli = Cli::parse();
  if let Some(shell) = cli.generate_shell() {
    generate_completion(shell)
  } else {
    print_app();
    initialize_panic_handler()?;
    initialize_logging()?;
    cli.run()
  }
}