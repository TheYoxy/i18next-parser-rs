use serde_json::Value;

use crate::{
  config::Config,
  log_time,
  merger::merge_results::{merge_results, MergeResults},
  transform::transform_entries::{transform_entries, TransformEntriesResult},
  visitor::Entry,
};

pub(crate) fn merge_all_values(entries: Vec<Entry>, config: &Config) -> color_eyre::Result<Vec<MergeResults>> {
  log_time!("Preparing entries to write", || {
    let locales = &config.locales;
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
              merge_results(locale, namespace, catalog, unique_count, unique_plurals_count, config)
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
  use crate::{config::Config, helper::merge_hashes::MergeResult, visitor::Entry};

  #[test]
  fn merge_all_values_simple_case() {
    let entries = vec![Entry {
      namespace: Some("default".into()),
      key: "key".into(),
      count: None,
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
        count: None,
        value: Some("value1".into()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".into()),
        key: "key2".into(),
        count: Some(3),
        value: Some("value2".into()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".into()),
        key: "key3".into(),
        count: None,
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
