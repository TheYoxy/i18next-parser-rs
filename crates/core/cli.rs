use std::{env, path::PathBuf};

use anstyle::Style;
use clap::{builder::Styles, command, Parser};
use clap_complete::Shell;

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
pub(crate) struct Cli {
  /// The path to extract the translations from
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string(), global = true, value_hint = clap::ValueHint::DirPath)]
  pub(crate) path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false")]
  pub(crate) verbose: bool,
  /// Should generate types
  #[arg(short, long, default_value = "false")]
  #[cfg(feature = "generate_types")]
  pub(crate) generate_types: bool,
  /// Should generate shell completions
  #[arg(long)]
  #[clap(value_enum)]
  pub(crate) generate_shell: Option<Shell>
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
