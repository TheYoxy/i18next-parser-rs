use std::collections::HashMap;

use serde_json::Value;

use crate::{config::Config, plural, printerror, transform::transform_entry::transform_entry, visitor::Entry};

pub(crate) struct TransformEntriesResult {
  pub(crate) unique_count: HashMap<String, usize>,
  pub(crate) unique_plurals_count: HashMap<String, usize>,
  pub(crate) value: Value,
  pub(crate) locale: String,
}

pub(crate) fn transform_entries(
  entries: &[Entry],
  locale: &str,
  config: &Config,
) -> color_eyre::Result<TransformEntriesResult> {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();

  let value = entries.iter().try_fold(Value::Object(Default::default()), |value, entry| {
    return if entry.has_count {
      let resolver = plural::PluralResolver::default();
      let suffixes = resolver.get_suffixes(locale);
      match suffixes {
        Ok(suffixes) => {
          suffixes.iter().try_fold(value, |value, suffix| {
            transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, Some(suffix))
          })
        },
        Err(e) => {
          printerror!("Error getting suffixes: {}", e);
          Ok(value)
        },
      }
    } else {
      transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, None)
    };
  })?;

  Ok(TransformEntriesResult { unique_count, unique_plurals_count, value, locale: locale.to_string() })
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;

  #[test]
  fn test_transform_entries() {
    let entries = vec![
      Entry {
        namespace: Some("default".to_string()),
        key: "key1".to_string(),
        has_count: false,
        value: Some("value1".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key2".to_string(),
        has_count: true,
        value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".to_string()),
        key: "key3".to_string(),
        has_count: false,
        value: Some("value3".to_string()),
        i18next_options: None,
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
    assert_eq!(
      result.value,
      json!({"default": {"key1": "value1","key2_one": "value2","key2_other": "value2",},"custom": {"key3": "value3",}})
    );
  }

  #[test]
  fn test_transform_entries_with_count_en() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    println!("{:?}", result.value);
    assert_eq!(
      result.value,
      json!({
      "default": {
          "key_one": "value",
          "key_other": "value",
        }
      })
    );
  }

  #[test]
  fn test_transform_entries_with_count_fr() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "fr";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&3));
    println!("{:?}", result.value);
    assert_eq!(
      result.value,
      json!({
      "default": {
          "key_one": "value",
          "key_many": "value",
          "key_other": "value",
        }
      })
    );
  }

  #[test]
  fn test_transform_entries_with_count_nl() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      has_count: true,
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "nl";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert!(result.is_ok());
    let result = result.unwrap();

    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    println!("{:?}", result.value);
    assert_eq!(
      result.value,
      json!({
      "default": {
          "key_one": "value",
          "key_other": "value",
        }
      })
    );
  }
}
