use std::collections::HashMap;

use serde_json::Value;

use crate::config::Config;
use crate::transform::transform_entry::transform_entry;
use crate::visitor::Entry;
use crate::{plural, printerror};

pub(crate) struct TransformEntriesResult {
  pub(crate) unique_count: HashMap<String, usize>,
  pub(crate) unique_plurals_count: HashMap<String, usize>,
  pub(crate) value: Value,
}

pub(crate) fn transform_entries(entries: &Vec<Entry>, locale: &str, config: &Config) -> TransformEntriesResult {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();
  let mut value = Value::Object(Default::default());

  for entry in entries {
    value = if entry.count.is_some() {
      let resolver = plural::PluralResolver::default();
      let suffixes = resolver.get_suffixes(locale);
      let mut value = Value::Object(Default::default());
      match suffixes {
        Ok(suffixes) => {
          for suffix in suffixes {
            value = transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, Some(&suffix))
          }
        },
        Err(e) => {
          printerror!("Error getting suffixes: {}", e)
        },
      }
      value
    } else {
      transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, None)
    };
  }
 value.as_object_mut().unwrap();
  TransformEntriesResult { unique_count, unique_plurals_count, value }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_transform_entries() {
    let entries = vec![
      Entry {
        namespace: Some("default".to_string()),
        key: "key1".to_string(),
        count: None,
        value: Some("value1".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key2".to_string(),
        count: Some(3),
        value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".to_string()),
        key: "key3".to_string(),
        count: None,
        value: Some("value3".to_string()),
        i18next_options: None,
      },
    ];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_count.get("custom"), Some(&1));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("custom"), Some(&0));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key1")), Some(&Value::String("value1".to_string())));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key2")), Some(&Value::String("value2".to_string())));
    assert_eq!(result.value.get("custom").and_then(|v| v.get("key3")), Some(&Value::String("value3".to_string())));
  }

  #[test]
  fn test_transform_entries_with_count_en() {
    let entries = vec![Entry {
      namespace: Some("default".to_string()),
      key: "key".to_string(),
      count: Some(3),
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "en";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

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
      count: Some(3),
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "fr";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

    assert_eq!(result.unique_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
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
      count: Some(3),
      value: Some("value".to_string()),
      i18next_options: None,
    }];
    let locale = "nl";
    let config = Default::default();

    let result = transform_entries(&entries, locale, &config);

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
