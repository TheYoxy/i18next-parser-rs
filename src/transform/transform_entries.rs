use std::collections::HashMap;

use serde_json::Value;

use crate::config::Config;
use crate::plural;
use crate::transform::transform_entry::transform_entry;
use crate::visitor::Entry;

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
    value = if config.plural_separator.is_some() && entry.count.is_some() {
      let resolver = plural::PluralResolver::default();

      for suffix in resolver.get_suffixes(locale) {
        value = transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, Some(&suffix))
      }
      value
    } else {
      transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, config, None)
    };
  }
  TransformEntriesResult { unique_count, unique_plurals_count, value }
}

#[cfg(test)]
mod transform_entries {
  use super::*;

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
}
