//! This module contains the logic to transform entries into a JSON object.
use std::collections::HashMap;

use crate::{config::Config, merger::merge_all_values::FoundValue, transform::transform_entry::transform_entry, Entry};

/// Represents the result of transforming entries.
pub struct TransformEntriesResult {
  /// The unique count of entries.
  pub unique_count: HashMap<String, usize>,
  /// The unique count of plural entries.
  pub unique_plurals_count: HashMap<String, usize>,
  /// The transformed value.
  pub value: FoundValue,
  /// The locale of the transformed value.
  pub locale: String,
}

/// Transforms entries into a JSON object.
///
/// # Arguments
///
/// * `entries` - A reference to the entries to transform.
/// * `locale` - The locale of the entries.
/// * `config` - A reference to the configuration.
///
/// # Returns
///
/// * `Result<TransformEntriesResult, color_eyre::Error>` - The result of transforming the entries.
pub fn transform_entries(
  entries: &[Entry],
  locale: &str,
  config: &Config,
) -> color_eyre::Result<TransformEntriesResult> {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();

  let value = entries.iter().fold(FoundValue::new(), |mut value, entry| {
    transform_entry(entry, &mut unique_count, &mut unique_plurals_count, config, locale, &mut value);
    value
  });

  Ok(TransformEntriesResult { unique_count, unique_plurals_count, value, locale: locale.to_string() })
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::{json, Map, Value};

  use super::*;
  use crate::Entry;

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries() {
    let entries = vec![
      Entry {
        namespace: Some("default".to_string()),
        key: "key1".to_string(),
        has_count: false,
        value: Some("value1".to_string()),
        ..Default::default()
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key2".to_string(),
        has_count: true,
        value: Some("value2".to_string()),
        ..Default::default()
      },
      Entry {
        namespace: Some("custom".to_string()),
        key: "key3".to_string(),
        has_count: false,
        value: Some("value3".to_string()),
        ..Default::default()
      },
    ];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_count.get("custom"), Some(&1));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("custom"), Some(&0));
    // assert_eq!(
    //   result.value,
    //   json!({"default": {"key1": "value1","key2_one": "value2","key2_other": "value2",},"custom": {"key3": "value3",}})
    // );
  }

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries_with_count_en() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      ..Default::default()
    }];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    println!("{:?}", result.value);
    // assert_eq!(
    //   result.value,
    //   json!({
    //   "default": {
    //       "key_one": "value",
    //       "key_other": "value",
    //     }
    //   })
    // );
  }

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries_with_multiple_context() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: false,
      value: Some("value".to_string()),
      context: Some(vec!["male".to_string(), "female".to_string()]),
      ..Default::default()
    }];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    println!("{:#?}", result.value);
    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&0));

    let map = Map::from_iter(result.value.iter().map(|(k, v)| (k.clone(), Value::String(v.value.clone()))));

    assert_eq!(
      map,
      *json!({
          "default.key_male": "value".to_string(),
          "default.key_female": "value".to_string()
      })
      .as_object()
      .unwrap()
    );
  }

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries_with_context() {
    let entries = vec![
      Entry {
        namespace: Some("default".to_string()),
        key: "key".to_string(),
        has_count: false,
        value: Some("male value".to_string()),
        context: Some(vec!["male".to_string()]),
        ..Default::default()
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key".to_string(),
        has_count: false,
        value: Some("female value".to_string()),
        context: Some(vec!["female".to_string()]),
        ..Default::default()
      },
    ];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    println!("{:#?}", result.value);
    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&0));

    let map = Map::from_iter(result.value.iter().map(|(k, v)| (k.clone(), Value::String(v.value.clone()))));

    assert_eq!(
      map,
      *json!({
          "default.key_male": "male value".to_string(),
          "default.key_female": "female value".to_string()
      })
      .as_object()
      .unwrap()
    );
  }

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries_with_count_fr() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      ..Default::default()
    }];
    let locale = "fr";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&3));
    println!("{:?}", result.value);
    // assert_eq!(
    //   result.value,
    //   json!({
    //   "default": {
    //       "key_one": "value",
    //       "key_many": "value",
    //       "key_other": "value",
    //     }
    //   })
    // );
  }

  #[test_log::test(ignore = "count are not correctly implemented")]
  fn test_transform_entries_with_count_nl() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      ..Default::default()
    }];
    let locale = "nl";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    println!("{:?}", result.value);
    // assert_eq!(
    //   result.value,
    //   json!({
    //   "default": {
    //       "key_one": "value",
    //       "key_other": "value",
    //     }
    //   })
    // );
  }
}
