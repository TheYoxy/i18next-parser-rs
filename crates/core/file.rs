use std::{
  fs::File,
  io::Write,
  path::{Path, PathBuf},
};

use color_eyre::Report;
use log::trace;
use serde_json::Value;

use crate::{
  config::Config, config::LineEnding, helper::merge_hashes::MergeResult, is_empty::IsEmpty, log_time,
  merger::merge_results::MergeResults,
};

/// Write all entries to the specific file based on its namespace
pub(crate) fn write_to_file<T: AsRef<Config>>(values: &[MergeResults], config: T) -> color_eyre::Result<()> {
  let config = config.as_ref();
  log_time!("Writing files", || {
    for value in values {
      let MergeResults { namespace: _namespace, locale: _locale, path, backup, merged, old_catalog } = value;
      write_files(path, backup, merged, old_catalog, config)?;
    }

    Ok(())
  })
}

fn write_files<T: AsRef<Config>>(
  path: &PathBuf,
  backup: &PathBuf,
  merged: &MergeResult,
  old_catalog: &Value,
  config: T,
) -> Result<(), Report> {
  let config = config.as_ref();
  log_time!(format!("Writing file {path:?}"), || {
    let new_catalog = &merged.new;
    push_file(path, new_catalog, config)?;
    if config.create_old_catalogs && !old_catalog.is_empty() {
      push_file(backup, old_catalog, config)?;
    }
    Ok(())
  })
}

fn push_file<T: AsRef<Config>>(path: &PathBuf, contents: &Value, config: T) -> std::io::Result<()> {
  fn handle_line_ending(text: &str, line_ending: &LineEnding) -> String {
    match line_ending {
      LineEnding::Crlf => text.replace('\n', "\r\n"),
      LineEnding::Cr => text.replace('\n', "\r"),
      _ => {
        // Do nothing, as Rust automatically uses the appropriate line endings
        text.to_string()
      },
    }
  }

  let text = {
    let text = if path.ends_with("yml") {
      serde_yaml::to_string(contents).unwrap()
    } else {
      serde_json::to_string_pretty(contents).map(|t| t.replace("\r\n", "\n").replace('\r', "\n")).unwrap()
    };

    handle_line_ending(&text, &config.as_ref().line_ending)
  };

  if let Some(parent) = path.parent() {
    if !parent.exists() {
      trace!("creating parent directory: {:?}", parent);
      std::fs::create_dir_all(parent)?;
    }
  }
  let mut file = File::create(Path::new(path))?;
  file.write_all(text.as_bytes())?;

  Ok(())
}