use std::{path::PathBuf, str::FromStr};

use color_eyre::owo_colors::OwoColorize;
use log::trace;
use serde_json::Value;

use crate::{
  config::Config,
  file::catalog::read_file_into_serde,
  helper::merge_hashes::{MergeResult, merge_hashes},
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
  is_default: bool,
  config: C,
) -> MergeResults {
  let config = config.as_ref();
  let path = config.get_output_dir(locale, namespace);
  trace!("Path for output {}", path.yellow());
  let path = PathBuf::from_str(&path).unwrap_or_else(|_| panic!("Unable to find path {path:?}"));
  // get backup file name
  let filename = {
    let filename = path.file_stem().and_then(|o| o.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|o| o.to_str()).unwrap_or_default();
    format!("{filename}_old.{extension}")
  };
  let backup = path.with_file_name(filename);

  trace!("File path: {}", path.display().yellow());
  trace!("Backup path: {}", backup.display().yellow());

  let value = read_file_into_serde(&path).inspect(|val| {
    trace!("From {}: {}", path.display().yellow(), val.cyan());
  });
  let value = value.as_ref();

  let old_value = read_file_into_serde(&backup).inspect(|val| {
    trace!("From {}: {}", backup.display().yellow(), val.cyan());
  });
  let old_value = old_value.as_ref();

  let full_key_prefix = format!("{}{}", namespace, config.namespace_separator);
  trace!("Merging value");
  let merged = merge_hashes(value, catalog, old_value, &full_key_prefix, is_default, locale, config);

  trace!("Merging old catalog");
  let old_merged = merge_hashes(
    old_value,
    &merged.new,
    None,
    &full_key_prefix,
    false,
    locale,
    &Config {
      keep_removed: false,
      ..Default::default()
    },
  );

  trace!("Building old catalog");
  let old_catalog = transfer_values(&merged.old, &old_merged.old);

  MergeResults {
    namespace: namespace.to_string(),
    locale: locale.to_string(),
    path,
    backup,
    merged,
    old_catalog,
  }
}

#[cfg(test)]
mod tests {
  use color_eyre::eyre::eyre;
  use log::{debug, info};
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use tempdir::TempDir;

  use super::*;

  #[allow(dead_code)]
  fn write_locale(dir: &TempDir, ns: &str, locale: &str, value: &Value) -> color_eyre::Result<String> {
    std::fs::create_dir_all(dir.path())?;
    let output = dir.path().join("locales").join(ns).join(format!("{locale}.json"));
    std::fs::create_dir_all(output.parent().unwrap())?;
    info!("Opening file {output:?}");
    let file = std::fs::File::create(&output)?;
    serde_json::to_writer_pretty(file, &value)?;
    debug!("Written {} to {}", value.cyan(), output.display().yellow());

    output
      .to_str()
      .ok_or(eyre!("Unable to get path"))
      .map(|s| s.to_string())
  }

  #[test_log::test]
  fn should_not_override_defaults() {
    let value = json!({
      "key": "default_value"
    });

    let locale = "en";
    let namespace = "default";
    let dir = TempDir::new("merge_results").unwrap();
    let output = write_locale(&dir, locale, namespace, &value).unwrap();
    let catalog = json!({
        "key": "value"
    });
    let is_default = true;
    let config = Config {
      locales: vec![locale.into()],
      output,
      ..Default::default()
    };

    let result = merge_results(locale, namespace, &catalog, is_default, config);
    let merged = result.merged;
    assert_eq!(merged.new, catalog, "the new value do not match");
    assert_eq!(merged.old, value, "the old value do not match");
    assert_eq!(merged.merged_count, 0, "the merge count do not match");
  }

  mod count {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test_log::test]
    fn should_not_override_existing_counts() {
      let value = json!({
        "key_one": "default_value",
        "key_many": "default_value"
      });

      let locale = "en";
      let namespace = "default";
      let dir = TempDir::new("merge_results").unwrap();
      let output = write_locale(&dir, locale, namespace, &value).unwrap();
      let catalog = json!({
          "key_one": "value",
          "key_many": "value"
      });
      let is_default = true;
      let config = Config {
        locales: vec![locale.into()],
        output,
        ..Default::default()
      };

      let result = merge_results(locale, namespace, &catalog, is_default, config);
      let merged = result.merged;
      assert_eq!(merged.new, value, "the new value do not match");
      assert_eq!(merged.old, json!({}), "the old value do not match");
      assert_eq!(merged.merged_count, 0, "the merge count do not match");
    }

    #[test_log::test]
    fn should_not_override_existing_context_but_add_missing_count() {
      let value = json!({
        "key_one": "default_value",
      });

      let locale = "en";
      let namespace = "default";
      let dir = TempDir::new("merge_results").unwrap();
      let output = write_locale(&dir, locale, namespace, &value).unwrap();
      let catalog = json!({
          "key_one": "value",
          "key_many": "default_value"
      });
      let is_default = true;
      let config = Config {
        locales: vec![locale.into()],
        output,
        ..Default::default()
      };

      let result = merge_results(locale, namespace, &catalog, is_default, config);
      let merged = result.merged;
      assert_eq!(
        merged.new,
        json!({
          "key_many": "default_value",
          "key_one": "default_value"
        }),
        "the new value should not be overridden when having a context"
      );
      assert_eq!(merged.old, json!({}), "the old value do not match");
      assert_eq!(merged.merged_count, 0, "the merge count do not match");
    }
  }

  mod context {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test_log::test]
    fn should_not_override_existing_context() {
      let value = json!({
        "key_male": "default_value",
        "key_female": "default_value"
      });

      let locale = "en";
      let namespace = "default";
      let dir = TempDir::new("merge_results").unwrap();
      let output = write_locale(&dir, locale, namespace, &value).unwrap();
      let catalog = json!({
          "key_male": "value",
          "key_female": "value"
      });
      let is_default = true;
      let config = Config {
        locales: vec![locale.into()],
        output,
        ..Default::default()
      };

      let result = merge_results(locale, namespace, &catalog, is_default, config);
      let merged = result.merged;
      assert_eq!(
        merged.new, value,
        "the new value should not be overridden when having a context"
      );
      assert_eq!(merged.old, json!({}), "the old value do not match");
      assert_eq!(merged.merged_count, 0, "the merge count do not match");
    }

    #[test_log::test]
    fn should_not_override_existing_context_but_add_missing_context() {
      let value = json!({
        "key_male": "default_value",
      });

      let locale = "en";
      let namespace = "default";
      let dir = TempDir::new("merge_results").unwrap();
      let output = write_locale(&dir, locale, namespace, &value).unwrap();
      let catalog = json!({
          "key_male": "value",
          "key_female": "default_value"
      });
      let is_default = true;
      let config = Config {
        locales: vec![locale.into()],
        output,
        ..Default::default()
      };

      let result = merge_results(locale, namespace, &catalog, is_default, config);
      let merged = result.merged;
      assert_eq!(
        merged.new,
        json!({
          "key_male": "default_value",
          "key_female": "default_value"
        }),
        "the new value should not be overridden when having a context"
      );
      assert_eq!(merged.old, json!({}), "the old value do not match");
      assert_eq!(merged.merged_count, 0, "the merge count do not match");
    }
  }
}
