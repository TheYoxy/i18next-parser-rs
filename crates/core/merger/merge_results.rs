use std::{collections::HashMap, path::PathBuf, str::FromStr};

use color_eyre::owo_colors::OwoColorize;
use log::trace;
use serde_json::Value;

use crate::{
  catalog::read_file_into_serde,
  config::Config,
  helper::merge_hashes::{merge_hashes, MergeResult},
  print::print_count::print_counts,
  transform::transfer_values::transfer_values,
};

#[derive(Debug, Default, Eq, PartialEq)]
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
  is_default: bool,
  config: C,
) -> MergeResults {
  let config = config.as_ref();
  let output = config.get_output();
  let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", namespace);
  trace!("Path for output {}: {}", output.yellow(), path.yellow());
  let path = PathBuf::from_str(&path).unwrap_or_else(|_| panic!("Unable to find path {path:?}"));
  // get backup file name
  let filename = {
    let filename = path.file_stem().and_then(|o| o.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|o| o.to_str()).unwrap_or_default();
    format!("{}_old.{}", filename, extension)
  };
  let backup = path.with_file_name(filename);
  trace!("File path: {}", path.display().yellow());
  trace!("Backup path: {}", backup.display().yellow());

  let value = read_file_into_serde(&path);

  let old_value = read_file_into_serde(&backup);
  let old_value = old_value.as_ref();

  trace!("Value: {:?} -> {:?}", value.cyan(), old_value.cyan());

  let full_key_prefix = format!("{}{}", namespace, config.key_separator);
  let merged = merge_hashes(catalog, value.as_ref(), old_value, &full_key_prefix, is_default, config);
  let old_merged = merge_hashes(&merged.new, old_value, None, &full_key_prefix, false, &Config {
    keep_removed: false,
    ..Default::default()
  });
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if config.verbose {
    print_counts(locale, namespace, unique_count, unique_plurals_count, &merged, &old_merged, config);
  }

  MergeResults { namespace: namespace.to_string(), locale: locale.to_string(), path, backup, merged, old_catalog }
}

#[cfg(test)]
mod tests {
  use color_eyre::eyre::eyre;
  use log::{debug, info};
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use tempdir::TempDir;

  use super::*;
  use crate::utils::initialize_logging;

  fn init_test(dir: &TempDir, ns: &str, locale: &str, value: &Value) -> color_eyre::Result<String> {
    let _ = initialize_logging();
    std::fs::create_dir_all(dir.path())?;
    let output = dir.path().join("locales").join(ns).join(format!("{locale}.json"));
    std::fs::create_dir_all(output.parent().unwrap())?;
    info!("Opening file {output:?}");
    let file = std::fs::File::create(&output)?;
    serde_json::to_writer_pretty(file, &value)?;
    debug!("Written {} to {}", value.cyan(), output.display().yellow());

    output.to_str().ok_or(eyre!("Unable to get path")).map(|s| s.to_string())
  }

  #[test]
  fn merge_results_should_not_override_defaults() {
    let value = json!({
      "key": "default_value"
    });

    let locale = "en";
    let namespace = "default";
    let dir = TempDir::new("merge_results").unwrap();
    let output = init_test(&dir, locale, namespace, &value).unwrap();
    let catalog = json!({
        "key": "value"
    });
    let unique_count = HashMap::<String, usize>::new();
    let unique_plurals_count = HashMap::<String, usize>::new();
    let is_default = true;
    let config = Config { locales: vec![locale.into()], output, ..Default::default() };

    let result = merge_results(locale, namespace, &catalog, &unique_count, &unique_plurals_count, is_default, config);
    drop(dir);
    println!("Results: {:#?}", result);
    let merged = result.merged;
    assert_eq!(merged.new, catalog, "the new value do not match");
    assert_eq!(merged.old, value, "the old value do not match");
    assert_eq!(merged.merge_count, 0, "the merge count do not match");
  }
}
