//! This module contains the logic to transform entries into a JSON object.
use crate::{Entry, config::Config, merger::merge_all_values::FoundValue, transform::transform_entry::transform_entry};

/// Represents the result of transforming entries.
pub struct TransformEntriesResult {
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
  let value = entries.iter().fold(FoundValue::new(), |mut value, entry| {
    transform_entry(entry, config, locale, &mut value);
    value
  });

  Ok(TransformEntriesResult {
    value,
    locale: locale.to_string(),
  })
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::*;
  use crate::{Entry, models::FoundEntry};

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

    let result = transform_entries(&entries, locale, &config)
      .map(|e| e.value)
      .expect("the result should be ok");

    assert_eq!(result.get("default.key_male"), Some(&FoundEntry::new("male value")));
    assert_eq!(result.get("default.key_female"), Some(&FoundEntry::new("female value")));
  }

  #[test_log::test]
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

    let result = transform_entries(&entries, locale, &config)
      .map(|e| e.value)
      .expect("the result should be ok");

    assert_eq!(result.get("default.key_male"), Some(&FoundEntry::new("value")));
    assert_eq!(result.get("default.key_female"), Some(&FoundEntry::new("value")));
  }

  #[test_log::test]
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

    let result = transform_entries(&entries, locale, &config)
      .map(|e| e.value)
      .expect("the result should be ok");

    assert_eq!(result.get("default.key_male"), Some(&FoundEntry::new("male value")));
    assert_eq!(result.get("default.key_female"), Some(&FoundEntry::new("female value")));
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
