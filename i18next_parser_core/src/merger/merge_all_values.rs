use color_eyre::eyre::eyre;
use serde_json::Value;
use tracing::instrument;

use crate::{
  config::Config,
  log_time,
  merger::merge_results::{merge_results, MergeResults},
  transform::transform_entries::{transform_entries, TransformEntriesResult},
  Entry,
};

/// Merges all translation values across different locales based on the provided entries and configuration.
///
/// This function processes a vector of `Entry` objects, each representing a translation entry, and merges
/// them into a structured format suitable for writing to JSON files. The merging process is influenced by
/// the configuration specified in `config`, particularly the locales to be considered.
///
/// # Arguments
///
/// * `entries` - A vector of `Entry` objects representing the translation entries to be merged.
/// * `config` - A reference to a `Config` object containing the configuration settings for the merging process,
///   including the locales to be processed.
///
/// # Returns
///
/// A `color_eyre::Result` containing either:
/// - On success: a vector of `MergeResults` objects, each representing the merged results for a specific locale,
///   including details such as the namespace, locale, paths to the resulting JSON files, and the merged content.
/// - On failure: an error, typically due to issues with processing the entries or configuration.
///
/// # Errors
///
/// This function returns an error if:
/// - No locales are found in the provided configuration.
/// - An error occurs during the transformation or merging of entries.
///
/// # Examples
///
/// ```
/// use i18next_parser_core::{merge_all_values, Config, Entry};
/// let entries = vec![Entry {
///   namespace: Some("default".into()),
///   key: "key".into(),
///   has_count: false,
///   value: Some("value".into()),
///   i18next_options: None,
/// }];
/// let config = Config { locales: vec!["en".into()], ..Default::default() };
///
/// let result = merge_all_values(entries, &config);
/// assert!(result.is_ok());
/// ```
#[instrument(skip_all, err, target = "instrument")]
pub fn merge_all_values(entries: Vec<Entry>, config: &Config) -> color_eyre::Result<Vec<MergeResults>> {
  log_time!("Preparing entries to write", {
    let locales = &config.locales;
    let default_locale = &config.locales.first().ok_or(eyre!("No locales found in the configuration."))?;
    let results = locales
      .iter()
      .map(|locale| transform_entries(&entries, locale, config))
      .collect::<color_eyre::Result<Vec<_>>>()?;

    let result = results
      .iter()
      .filter_map(|entry| {
        let TransformEntriesResult { unique_count, unique_plurals_count, value, locale } = entry;

        if let Value::Object(catalog) = value {
          let result = catalog
            .iter()
            .map(|(namespace, catalog)| {
              merge_results(
                locale,
                namespace,
                catalog,
                unique_count,
                unique_plurals_count,
                locale == *default_locale,
                config,
              )
            })
            .collect::<Vec<_>>();
          Some(result)
        } else {
          None
        }
      })
      .flatten()
      .collect::<Vec<_>>();

    Ok(result)
  })
}
#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;
  use crate::{config::Config, helper::merge_hashes::MergeResult, Entry};

  #[test]
  fn merge_all_values_simple_case() {
    let entries = vec![Entry {
      namespace: Some("default".into()),
      key: "key".into(),
      has_count: false,
      value: Some("value".into()),
      i18next_options: None,
    }];
    let config = Config { locales: vec!["en".into()], ..Default::default() };

    let result = merge_all_values(entries, &config);

    assert!(result.is_ok());
    let result = result.unwrap();
    let expected: Vec<MergeResults> = vec![MergeResults {
      namespace: "default".into(),
      locale: "en".into(),
      path: "./locales/en/default.json".into(),
      backup: "./locales/en/default_old.json".into(),
      merged: MergeResult {
        new: json!({"key": "value"}),
        old: json!({}),
        reset: json!({}),
        merge_count: 0,
        pull_count: 0,
        old_count: 0,
        reset_count: 0,
      },
      old_catalog: json!({}),
    }];
    assert_eq!(result, expected);
  }

  #[test]
  fn merge_all_values_with_valid_entries_and_config() {
    let entries = vec![
      Entry {
        namespace: Some("default".into()),
        key: "key1".into(),
        has_count: false,
        value: Some("value1".into()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".into()),
        key: "key2".into(),
        has_count: true,
        value: Some("value2".into()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".into()),
        key: "key3".into(),
        has_count: false,
        value: Some("value3".into()),
        i18next_options: None,
      },
    ];
    let config = Config { locales: vec!["en".into()], ..Default::default() };

    let result = merge_all_values(entries, &config);

    assert!(result.is_ok());
    let result = result.unwrap();
    let expected: Vec<MergeResults> = vec![
      MergeResults {
        namespace: "custom".into(),
        locale: "en".into(),
        path: "./locales/en/custom.json".into(),
        backup: "./locales/en/custom_old.json".into(),
        merged: MergeResult {
          new: json!({"key3": "value3",}),
          old: json!({}),
          reset: json!({}),
          merge_count: 0,
          pull_count: 0,
          old_count: 0,
          reset_count: 0,
        },
        old_catalog: json!({}),
      },
      MergeResults {
        namespace: "default".into(),
        locale: "en".into(),
        path: "./locales/en/default.json".into(),
        backup: "./locales/en/default_old.json".into(),
        merged: MergeResult {
          new: json!({"key1": "value1", "key2_one": "value2","key2_other": "value2",}),
          old: json!({}),
          reset: json!({}),
          merge_count: 0,
          pull_count: 0,
          old_count: 0,
          reset_count: 0,
        },
        old_catalog: json!({}),
      },
    ];
    assert_eq!(result, expected);
  }

  #[test]
  fn merge_all_values_with_empty_entries() {
    let entries = vec![];
    let config = Config { locales: vec!["en".into()], ..Default::default() };

    let result = merge_all_values(entries, &config);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
  }
}
