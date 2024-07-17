//! This module contains the merge_hashes function that merges two JSON objects (hashes) together.
use color_eyre::owo_colors::OwoColorize;
use log::{debug, error, trace};
use regex::Regex;
use serde_json::{Map, Value};

use crate::config::Config;

const PLURAL_SUFFIXES: &[&str] = &["zero", "one", "two", "few", "many", "other"];

fn is_plural(key: &str) -> bool {
  PLURAL_SUFFIXES.iter().any(|suffix| key.ends_with(suffix))
}

fn has_related_plural_key(raw_key: &str, source: &Map<String, Value>) -> bool {
  PLURAL_SUFFIXES.iter().any(|suffix| source.contains_key(&format!("{}{}", raw_key, suffix)))
}

fn get_singular_form(key: &str, plural_separator: &str) -> String {
  let plural_regex = Regex::new(&format!(r"(\{}(?:zero|one|two|few|many|other))$", plural_separator)).unwrap();
  plural_regex.replace(key, "").to_string()
}


#[derive(Debug, Default, Eq, PartialEq)]
pub struct MergeResult {
  /// The merged hash
  pub new: Value,
  /// The old hash
  pub old: Value,
  pub reset: Value,
  pub merge_count: usize,
  pub pull_count: usize,
  pub old_count: usize,
  pub reset_count: usize,
}

/// Merges two JSON objects (hashes) together.
///
/// This function takes an existing JSON object and merges it with a source JSON object.
/// If a key exists in both, the value from the source object is used.
/// If a key exists only in the source object, it is added to the existing object.
/// The function also handles nested JSON objects.
///
/// # Arguments
///
/// * `existing_values` - A JSON object that will be updated with the values from the source object.
/// * `source` - An optional JSON object that contains the new values.
/// * `reset_values` - An optional JSON object that contains values to be reset.
/// * `full_key_prefix` - A string that is used as a prefix for the keys in the source object.
/// * `reset_and_flag` - A boolean that indicates whether the values should be reset and flagged.
/// * `config` - A reference to a Config object that contains configuration options.
///
/// # Returns
///
/// * `MergeResult` - A struct that contains the new JSON object, the old JSON object, the reset JSON object,
///   and counts of merged, pulled, old, and reset keys.
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use i18next_parser_core::merge_hashes;
/// let existing = json!({
///   "key1": "value1",
///   "key2": "value2"
/// });
/// let raw_source = json!({
///    "key1": "new_value1",
///    "key3": "value3"
/// });
/// let source = Some(&raw_source);
/// let config = Default::default();
/// let reset_values = None;
///
/// let result = merge_hashes(&existing, source, reset_values, "", false, &config);
///
/// assert_eq!(
///   result.new,
///   json!({
///     "key1": "new_value1",
///     "key2": "value2",
///   }),
///   "the new hash is not as expected"
/// );
/// ```
pub fn merge_hashes(
  existing_values: &Value,
  source: Option<&Value>,
  reset_values: Option<&Value>,
  full_key_prefix: &str,
  reset_and_flag: bool,
  config: &Config,
) -> MergeResult {
  let mut old = Map::new();
  let mut reset = Map::new();
  let mut merge_count = 0;
  let mut pull_count = 0;
  let mut old_count = 0;
  let mut reset_count = 0;
  let mut existing = existing_values.as_object().map_or_else(Map::new, |v| v.clone());

  // if source.is_none() {
  //   debug!("No source provided, returning existing hash as is. {:?}", existing.cyan());
  //   return MergeResult {
  //     new: Value::Object(existing),
  //     old: Value::Object(Map::new()),
  //     reset: Value::Object(Map::new()),
  //     merge_count: 0,
  //     pull_count: 0,
  //     old_count: 0,
  //     reset_count: 0,
  //   };
  // }

  let key_separator = &config.key_separator;
  let plural_separator = &config.plural_separator;
  let reset_values_map = reset_values.and_then(|v| v.as_object()).map_or_else(Map::new, |v| v.clone());

  if let Some(Value::Object(source_map)) = source {
    for (key, value) in source_map {
      trace!("Handling {} with value {}", key.italic().purple(), value.cyan());
      match existing.get_mut(key) {
        Some(target_value) if target_value.is_object() && value.is_object() => {
          debug!("Merging nested key: {}", key.yellow());
          let nested_result = merge_hashes(
            target_value,
            Some(value),
            reset_values_map.get(key),
            &format!("{full_key_prefix}{key}{key_separator}"),
            reset_and_flag,
            config,
          );
          *target_value = nested_result.new;
          merge_count += nested_result.merge_count;
          pull_count += nested_result.pull_count;
          old_count += nested_result.old_count;
          reset_count += nested_result.reset_count;
          match nested_result.old {
            Value::Object(old_map) if !old_map.is_empty() => {
              old.insert(key.clone(), old_map.into());
            },
            Value::Object(old_map) => {
              old = old_map;
            },
            _ => {
              error!("Old map is not an object: {:?}", nested_result.old);
            },
          }

          match nested_result.reset {
            Value::Object(reset_map) if !reset_map.is_empty() => {
              reset.insert(key.clone(), reset_map.into());
            },
            Value::Object(reset_map) => {
              reset = reset_map;
            },
            _ => {
              error!("reset map is not an object: {:?}", nested_result.reset);
            },
          }
        },
        Some(target_value)
          if reset_and_flag && !is_plural(key) && value != target_value || reset_values_map.contains_key(key) =>
        {
          debug!("Inserting key: {} with {}", key.purple(), value.cyan());
          old.insert(key.clone(), value.clone());
          old_count += 1;
          reset.insert(key.clone(), Value::Bool(true));
          reset_count += 1;
        },
        Some(target_value) => {
          trace!("Replacing key: {} from {} to {}", key.purple(), target_value.cyan(), value.cyan());
          *target_value = value.clone();
          merge_count += 1;
        },
        None => {
          debug!("Pulling key: {}", key.purple());
          let singular_key = get_singular_form(key, plural_separator);
          let plural_match = key != &singular_key;
          let context_match = singular_key.contains('_');
          let raw_key = singular_key.replace('_', "");

          if (context_match && existing.contains_key(&raw_key))
            || (plural_match && has_related_plural_key(&format!("{}{}", singular_key, plural_separator), &existing))
          {
            existing.insert(key.clone(), value.clone());
            pull_count += 1;
          } else {
            if config.keep_removed {
              existing.insert(key.clone(), value.clone());
            } else {
              old.insert(key.clone(), value.clone());
            }
            old_count += 1;
          }
        },
      }
      trace!("Existing: {:?}", existing.cyan());
    }
  } else {
    debug!("No source provided, returning existing hash as is. {:?}", existing.cyan());
  }

  MergeResult {
    new: Value::Object(existing),
    old: Value::Object(old),
    reset: Value::Object(reset),
    merge_count,
    pull_count,
    old_count,
    reset_count,
  }
}

#[cfg(test)]
mod tests {

  use serde_json::json;

  use super::*;

  #[test]
  fn test_merge_hashes_no_source() {
    let existing = json!({
      "key1": "value1",
      "key2": "value2"
    });
    let config = Default::default();
    let reset_values = None;

    let result = merge_hashes(&existing, None, reset_values, "", false, &config);

    assert_eq!(result.new, existing);
    assert_eq!(result.old, json!({}));
    assert_eq!(result.reset, json!({}));
    assert_eq!(result.merge_count, 0);
    assert_eq!(result.pull_count, 0);
    assert_eq!(result.old_count, 0);
    assert_eq!(result.reset_count, 0);
  }

  #[test]
  fn test_merge_hashes_with_source() {
    let value = json!({
      "key1": "new_value1",
      "key3": "value3"
    });
    let source = Some(&value);
    let existing = json!({
      "key1": "value1",
      "key2": "value2"
    });
    let config = Default::default();
    let reset_values = None;

    let result = merge_hashes(&existing, source, reset_values, "", false, &config);

    assert_eq!(
      result.new,
      json!({
        "key1": "new_value1",
        "key2": "value2",
      }),
      "the new hash is not as expected"
    );
    assert_eq!(
      result.old,
      json!({
        "key3": "value3"
      })
    );
    assert_eq!(result.reset, json!({}));
    assert_eq!(result.merge_count, 1);
    assert_eq!(result.pull_count, 0);
    assert_eq!(result.old_count, 1);
    assert_eq!(result.reset_count, 0);
  }

  #[test]
  fn test_merge_hashes_with_source_keep_old_values() {
    let value = json!({
      "key1": "new_value1",
      "key3": "value3"
    });
    let source = Some(&value);
    let existing = json!({
      "key1": "value1",
      "key2": "value2"
    });
    let config = Config { keep_removed: true, ..Default::default() };
    let reset_values = None;

    let result = merge_hashes(&existing, source, reset_values, "", false, &config);

    assert_eq!(
      result.new,
      json!({
        "key1": "new_value1",
        "key2": "value2",
        "key3": "value3"
      }),
      "the new hash is not as expected"
    );
    assert_eq!(result.old, json!({}));
    assert_eq!(result.reset, json!({}));
    assert_eq!(result.merge_count, 1);
    assert_eq!(result.pull_count, 0);
    assert_eq!(result.old_count, 1);
    assert_eq!(result.reset_count, 0);
  }
}
