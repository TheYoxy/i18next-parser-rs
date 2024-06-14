use std::fmt::Display;

use log::{debug, info, trace};
use regex::Regex;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::visitor::Entry;

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

pub fn merge_hashes(
  source: Option<&Value>,
  existing: &Value,
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
  let mut existing = existing.as_object().map_or_else(Map::new, |v| v.clone());

  if source.is_none() {
    debug!("No source provided, returning existing hash as is. {:?}", existing);
    return MergeResult {
      new: Value::Object(existing),
      old: Value::Object(Map::new()),
      reset: Value::Object(Map::new()),
      merge_count: 0,
      pull_count: 0,
      old_count: 0,
      reset_count: 0,
    };
  }

  let key_separator = config.key_separator.as_deref().unwrap_or("");
  let plural_separator = config.plural_separator.as_deref().unwrap_or("_");

  let reset_values_map = reset_values.and_then(|v| v.as_object()).map_or_else(Map::new, |v| v.clone());

  if let Some(Value::Object(source_map)) = source {
    for (key, value) in source_map {
      trace!("Handling {key:?} with value {value:?}");
      match existing.get_mut(key) {
        Some(target_value) if target_value.is_object() && value.is_object() => {
          trace!("Merging nested key: {}", key);
          let nested_result = merge_hashes(
            Some(value),
            target_value,
            reset_values_map.get(key),
            &format!("{}{}{}", full_key_prefix, key, key_separator),
            reset_and_flag,
            config,
          );
          merge_count += nested_result.merge_count;
          pull_count += nested_result.pull_count;
          old_count += nested_result.old_count;
          reset_count += nested_result.reset_count;
          if !nested_result.old.as_object().unwrap().is_empty() {
            old.insert(key.clone(), nested_result.old);
          }
          if !nested_result.reset.as_object().unwrap().is_empty() {
            reset.insert(key.clone(), nested_result.reset);
          }
        },
        Some(target_value)
          if reset_and_flag && !is_plural(key) && value != target_value || reset_values_map.contains_key(key) =>
        {
          debug!("Merging nested key: {}", key);
          old.insert(key.clone(), value.clone());
          old_count += 1;
          reset.insert(key.clone(), Value::Bool(true));
          reset_count += 1;
        },
        Some(target_value) => {
          *target_value = value.clone();
          merge_count += 1;
        },
        None => {
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
    }
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

#[derive(Debug)]
pub struct DotPathToHashResult {
  pub target: Value,
  pub duplicate: bool,
  pub conflict: Option<String>,
}

/// Converts an entry with a dot path to a hash.
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub fn dot_path_to_hash(entry: &Entry, target: &Value, suffix: Option<&str>, config: &Config) -> DotPathToHashResult {
  let mut conflict: Option<String> = None;
  let mut duplicate = false;
  let mut target = target.clone();
  let separator = config.key_separator.clone().unwrap_or(".".to_string());
  let base_path =
    entry.namespace.clone().or(Some("default".to_string())).map(|ns| ns + &separator + &entry.key).unwrap();
  let mut path =
    base_path.replace(r#"\\n"#, "\\n").replace(r#"\\r"#, "\\r").replace(r#"\\t"#, "\\t").replace(r#"\\\\"#, "\\");
  if let Some(suffix) = suffix {
    path += suffix;
  }
  trace!("Path: {:?}", path);

  if path.ends_with(&separator) {
    trace!("Removing trailing separator from path: {:?}", path);
    path = path[..path.len() - separator.len()].to_string();
    trace!("New path: {:?}", path);
  }

  let segments: Vec<&str> = path.split(&separator).collect();
  trace!("Val {:?} {:?} {:?}", &target, entry.key, entry.default_value);

  let mut inner = &mut target;
  #[allow(clippy::needless_range_loop)]
  for i in 0..segments.len() - 1 {
    let segment = segments[i];
    if !segment.is_empty() {
      if inner[segment].is_string() {
        conflict = Some("key".to_string());
      }
      if inner[segment].is_null() || conflict.is_some() {
        inner[segment] = Value::Object(Map::new());
      }
      inner = &mut inner[segment];
    }
  }

  let last_segment = segments[segments.len() - 1];
  let old_value = inner[last_segment].as_str().map(|s| s.to_owned());
  let new_value = entry
    .default_value
    .clone()
    .map(|new_value| {
      if let Some(old_value) = old_value {
        trace!("Values {:?} -> {:?}", old_value, new_value);
        if old_value != new_value && !old_value.is_empty() {
          if new_value.is_empty() {
            old_value
          } else {
            conflict = Some("value".to_string());
            duplicate = true;
            new_value
          }
        } else {
          new_value
        }
      } else {
        new_value
      }
    })
    .unwrap_or_default();

  #[allow(unused_variables)]
  #[allow(unreachable_code)]
  if let Some(custom_value_template) = &config.custom_value_template {
    todo!("validate the behavior of custom_value_template");
    inner[last_segment] = Value::Object(Map::new());
    if let Value::Object(map) = custom_value_template {
      for (key, value) in map {
        if value == "${defaultValue}" {
          inner[last_segment][key] = Value::String(new_value.clone());
        } else {
          let value_key = value.as_str().unwrap().replace("${", "").replace('}', "");
          inner[last_segment][key] = Value::String(value_key);
        }
      }
    }
  } else {
    debug!("Setting {path:?} -> {new_value:?}");
    inner[last_segment] = Value::String(new_value);
  }

  DotPathToHashResult { target: target.clone(), duplicate, conflict }
}

pub fn log_execution_time<S, F, R>(message: S, func: F) -> R
where
  S: Display,
  F: FnOnce() -> R,
{
  use log::info;
  use std::time::Instant;
  let start = Instant::now();
  let result = func();
  let duration = start.elapsed();
  let duration_ms = duration.as_secs_f64() * 1000.0;
  info!("{} - Execution time: {:.2} ms", message, duration_ms);
  result
}

#[cfg(test)]
mod dot_path_to_hash {

  use serde_json::json;

  use super::*;

  #[test]
  fn test_dot_path_to_hash() {
    let entry = Entry {
      namespace: Some("namespace".to_string()),
      key: "key".to_string(),
      default_value: Some("default_value".to_string()),
      i18next_options: None,
      count: None,
    };
    let target = json!({
      "namespace": {
        "key": "existing_value"
      }
    });
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &target, None, &config);

    assert_eq!(
      result.target,
      json!({
        "namespace": {
          "key": "default_value"
        }
      })
    );
    assert!(result.duplicate, "there is not duplicates");
    assert_eq!(result.conflict, Some("value".to_string()));
  }
}

#[cfg(test)]
mod merge_hashes {

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

    let result = merge_hashes(None, &existing, reset_values, "", false, &config);

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

    let result = merge_hashes(source, &existing, reset_values, "", false, &config);

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

    let result = merge_hashes(source, &existing, reset_values, "", false, &config);

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
