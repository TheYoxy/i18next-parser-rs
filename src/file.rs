use std::{
  collections::HashMap,
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  str::FromStr,
};

use color_eyre::Report;
use log::trace;
use serde_json::Value;

use crate::{
  catalog::get_catalog,
  config::Config,
  config::LineEnding,
  helper::merge_hashes::{merge_hashes, MergeResult},
  is_empty::IsEmpty,
  log_time,
  print::print_count::print_counts,
  transform::transfer_values::transfer_values,
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

pub(crate) struct MergeResults {
  pub(crate) namespace: String,
  pub(crate) locale: String,
  pub(crate) path: PathBuf,
  pub(crate) backup: PathBuf,
  pub(crate) merged: MergeResult,
  pub(crate) old_catalog: Value,
}

pub(crate) fn merge_results<C: AsRef<Config>>(
  locale: &str,
  namespace: &str,
  catalog: &Value,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  config: C,
) -> MergeResults {
  let config = config.as_ref();
  let output = config.get_output();
  let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", namespace);
  trace!("Path for output {output:?}: {path:?}");
  let path = PathBuf::from_str(&path).unwrap_or_else(|_| panic!("Unable to find path {path:?}"));
  // get backup file name
  let filename = {
    let filename = path.file_stem().and_then(|o| o.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|o| o.to_str()).unwrap_or_default();
    format!("{}_old.{}", filename, extension)
  };
  let backup = path.with_file_name(filename);
  trace!("File path: {path:?}");
  trace!("Backup path: {backup:?}");

  let value = get_catalog(&path);

  let old_value = get_catalog(&backup);
  let old_value = old_value.as_ref();

  trace!("Value: {value:?} -> {old_value:?}");

  let ns_separator = &config.key_separator;
  let full_key_prefix = namespace.to_string() + ns_separator;
  let merged = merge_hashes(catalog, value.as_ref(), old_value, &full_key_prefix, false, config);
  let old_merged = merge_hashes(
    &merged.new,
    old_value,
    None,
    &full_key_prefix,
    false,
    &Config { keep_removed: false, ..Default::default() },
  );
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if config.verbose {
    print_counts(locale, namespace, unique_count, unique_plurals_count, &merged, &old_merged, config);
  }

  MergeResults { namespace: namespace.to_string(), locale: locale.to_string(), path, backup, merged, old_catalog }
}

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

fn push_file<T: AsRef<Config>>(path: &PathBuf, contents: &Value, config: T) -> std::io::Result<()> {
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
