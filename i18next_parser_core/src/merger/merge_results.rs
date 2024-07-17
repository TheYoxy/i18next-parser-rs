use std::{collections::HashMap, path::PathBuf, str::FromStr};

use color_eyre::owo_colors::OwoColorize;
use log::trace;
use serde_json::Value;

use crate::{
  config::Config,
  file::catalog::read_file_into_serde,
  helper::merge_hashes::{merge_hashes, MergeResult},
  print::print_count::print_counts,
  transform::transfer_values::transfer_values,
};

/// Represents the results of merging translation files.
///
/// # Fields
/// - `namespace`: The namespace of the translation, used to categorize translations.
/// - `locale`: The locale of the translation, representing the language and possibly region.
/// - `path`: The path to the merged translation file.
/// - `backup`: The path to the backup of the original translation file before merging.
/// - `merged`: The result of the merge operation, including counts of new, removed, and unchanged translations.
/// - `old_catalog`: The original translation data before the merge.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct MergeResults {
  /// The namespace of the translation, used to categorize translations.
  pub namespace: String,
  /// The locale of the translation, representing the language and possibly region.
  pub locale: String,
  /// The path to the merged translation file.
  pub path: PathBuf,
  /// The path to the backup of the original translation file before merging.
  pub backup: PathBuf,
  /// The result of the merge operation, including counts of new, removed, and unchanged translations.
  pub merged: MergeResult,
  /// The original translation data before the merge.
  pub old_catalog: Value,
}

/// Merges translation data from different sources and produces a `MergeResults` struct.
///
/// This function takes the current and new translation data, along with configuration options,
/// and merges them according to the specified rules. It handles file paths, backup creation,
/// and logging of the merge process.
///
/// # Parameters
/// - `locale`: The locale of the translations to merge.
/// - `namespace`: The namespace of the translations to merge.
/// - `catalog`: The new translation data to merge into the existing data.
/// - `unique_count`: A map of unique translation keys and their counts.
/// - `unique_plurals_count`: A map of unique plural translation keys and their counts.
/// - `is_default`: A flag indicating if the default translations are being merged.
/// - `config`: The configuration settings for the merge operation.
///
/// # Returns
/// A `MergeResults` struct containing the results of the merge operation.
pub fn merge_results<C: AsRef<Config>>(
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

  fn init_test(dir: &TempDir, ns: &str, locale: &str, value: &Value) -> color_eyre::Result<String> {
    std::fs::create_dir_all(dir.path())?;
    let output = dir.path().join("locales").join(ns).join(format!("{locale}.json"));
    std::fs::create_dir_all(output.parent().unwrap())?;
    info!("Opening file {output:?}");
    let file = std::fs::File::create(&output)?;
    serde_json::to_writer_pretty(file, &value)?;
    debug!("Written {} to {}", value.cyan(), output.display().yellow());

    output.to_str().ok_or(eyre!("Unable to get path")).map(|s| s.to_string())
  }

  #[test_log::test]
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
