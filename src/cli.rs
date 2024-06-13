use std::{env, path::PathBuf};

use clap::{command, Parser};

fn get_default_log_path() -> PathBuf {
  env::current_exe().unwrap()
}

#[derive(Parser, Debug)]
#[command(version, about, author)]
pub struct Cli {
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string())]
  pub path: PathBuf,

  /// Should the output to be verbose
  #[arg(short, long, default_value = "false")]
  pub verbose: bool,
}
