use std::collections::HashMap;

use log::trace;
use serde_json::Value;

use crate::{config::Options, helper::dot_path_to_hash, plural, printwarn, visitor::Entry};

pub fn transfer_values(source: &Value, target: &Value) -> Value {
  if let (Value::Object(source_map), Value::Object(target_map)) = (source, target) {
    let mut new_target_map = target_map.clone();
    for (key, source_value) in source_map {
      if !new_target_map.contains_key(key) {
        new_target_map.insert(key.clone(), source_value.clone());
      } else {
        let target_value = new_target_map.get_mut(key).unwrap();
        let transferred_value = transfer_values(source_value, target_value);
        *target_value = transferred_value;
      }
    }
    Value::Object(new_target_map)
  } else {
    target.clone()
  }
}

pub struct TransformEntriesResult {
  pub unique_count: HashMap<String, usize>,
  pub unique_plurals_count: HashMap<String, usize>,
  pub value: Value,
}

pub fn transform_entries(entries: &Vec<Entry>, locale: &str, options: &Options) -> TransformEntriesResult {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();
  let mut value = Value::Object(Default::default());

  for entry in entries {
    value = if options.plural_separator.is_some() && entry.count.is_some() {
      let resolver = plural::PluralResolver::default();

      for suffix in resolver.get_suffixes(locale) {
        value = transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, options, Some(suffix))
      }
      value
    } else {
      transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, options, None)
    };
  }
  TransformEntriesResult { unique_count, unique_plurals_count, value }
}

pub fn transform_entry(
  entry: &Entry,
  unique_count: &mut HashMap<String, usize>,
  unique_plurals_count: &mut HashMap<String, usize>,
  value: &Value,
  options: &Options,
  suffix: Option<String>,
) -> Value {
  let namespace = entry.namespace.clone().unwrap_or("default".to_string());
  if !unique_count.contains_key(&namespace) {
    unique_count.insert(namespace.clone(), 0);
  }
  if !unique_plurals_count.contains_key(&namespace) {
    unique_plurals_count.insert(namespace.clone(), 0);
  }

  let options = if let Some(suffix) = &suffix {
    Options { suffix: Some(suffix.clone()), ..options.clone() }
  } else {
    options.clone()
  };
  let result = dot_path_to_hash(entry, value, &options);
  trace!("Result: {:?} <- {:?}", value, result.target);

  if result.duplicate {
    match result.conflict {
      Some(val) if val == "key" => printwarn!(
        "Found translation key already mapped to a map or parent of new key already mapped to a string: {}",
        entry.key
      ),
      Some(val) if val == "value" => printwarn!("Found same keys with different values: {}", entry.key),
      _ => (),
    }
  } else {
    *unique_count.get_mut(&namespace).unwrap() += 1;
    if suffix.is_some() {
      *unique_plurals_count.get_mut(&namespace).unwrap() += 1;
    }
  }

  result.target
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
        default_value: Some("value1".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key2".to_string(),
        count: Some(3),
        default_value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".to_string()),
        key: "key3".to_string(),
        count: None,
        default_value: Some("value3".to_string()),
        i18next_options: None,
      },
    ];
    let locale = "en";
    let options = Options { plural_separator: Some("_".to_string()), ..Options::default() };

    let result = transform_entries(&entries, locale, &options);

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_count.get("custom"), Some(&1));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("custom"), Some(&0));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key1")), Some(&Value::String("value1".to_string())));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key2")), Some(&Value::String("value2".to_string())));
    assert_eq!(result.value.get("custom").and_then(|v| v.get("key3")), Some(&Value::String("value3".to_string())));
  }
}

#[cfg(test)]
mod transform_entry {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_transform_entry() {
    let entry = Entry {
      namespace: Some("default".to_string()),
      key: "key1".to_string(),
      default_value: Some("value1".to_string()),
      count: None,
      i18next_options: None,
    };
    let mut unique_count = HashMap::new();
    let mut unique_plurals_count = HashMap::new();
    let value = Value::Object(Default::default());
    let options = Options::default();

    let result = transform_entry(&entry, &mut unique_count, &mut unique_plurals_count, &value, &options, None);

    assert_eq!(
      result,
      json!({
        "default": {
          "key1": "value1"
        }
      })
    );
    assert_eq!(unique_count.get("default"), Some(&1));
    assert_eq!(unique_plurals_count.get("default"), Some(&0));
  }
}
