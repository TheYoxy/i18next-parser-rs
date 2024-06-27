use std::{env, path::PathBuf};

use clap::{command, Parser};

fn get_default_log_path() -> PathBuf {
  env::current_dir().unwrap()
}

#[derive(Parser, Debug)]
#[command(version, about, author)]
pub(crate) struct Cli {
  /// The path to extract the translations from
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string())]
  pub(crate) path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false")]
  pub(crate) verbose: bool,
  /// Should generate types
  #[arg(short, long, default_value = "false")]
  pub(crate) generate_types: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn should_parse_cli() {
    let cli = Cli::parse();
    assert_eq!(cli.path, get_default_log_path());
    assert!(!cli.verbose);
  }
}
