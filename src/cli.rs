use std::{env, path::PathBuf};

use clap::{command, Parser};
use log::LevelFilter;

fn get_default_log_path() -> PathBuf {
  env::current_exe().unwrap()
}

#[derive(Parser, Debug)]
#[command(version, about, author)]
pub struct Cli {
  #[arg(value_name = "PATH", default_value = get_default_log_path().into_os_string())]
  pub path: PathBuf,

  #[arg(short, long, default_value_t = LevelFilter::Info)]
  pub log_level: LevelFilter,
}
