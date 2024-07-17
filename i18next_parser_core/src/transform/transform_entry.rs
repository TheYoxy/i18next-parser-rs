use std::collections::HashMap;

use color_eyre::{eyre::bail, owo_colors::OwoColorize};
use log::{trace, warn};
use serde_json::Value;

use crate::{
  config::Config,
  helper::{
    dot_path_to_hash::{dot_path_to_hash, Conflict},
    get_char_diff::get_char_diff,
  },
  Entry,
};

pub fn transform_entry(
  entry: &Entry,
  unique_count: &mut HashMap<String, usize>,
  unique_plurals_count: &mut HashMap<String, usize>,
  value: &Value,
  options: &Config,
  suffix: Option<&str>,
) -> color_eyre::Result<Value> {
  let namespace = entry.namespace.clone().unwrap_or("default".to_string());
  if !unique_count.contains_key(&namespace) {
    unique_count.insert(namespace.clone(), 0);
  }
  if !unique_plurals_count.contains_key(&namespace) {
    unique_plurals_count.insert(namespace.clone(), 0);
  }

  let result = dot_path_to_hash(entry, value, suffix, options);
  trace!("Result: {} <- {}", value.cyan(), result.target.cyan());

  match result.conflict {
    Some(Conflict::Key(key)) => {
      warn!("Found translation key already mapped to a map or parent of new key already mapped to a string: {key}");
      if options.fail_on_warnings {
        bail!("Found translation key already mapped to a map or parent of new key already mapped to a string: {key}")
      }
    },
    Some(Conflict::Value(old, new)) => {
      let separator: &str = options.namespace_separator.as_ref();
      let diff = get_char_diff(&old, &new);
      warn!(
        "Found same keys with different values: {namespace}{separator}{key}: {diff}",
        namespace = namespace.bright_yellow(),
        key = entry.key.blue(),
        diff = diff
      );
    },
    _ => {
      *unique_count.get_mut(&namespace).unwrap() += 1;
      if suffix.is_some() {
        *unique_plurals_count.get_mut(&namespace).unwrap() += 1;
      }
    },
  }

  Ok(result.target)
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_transform_entry() {
    let entry = Entry {
      namespace: Some("default".to_string()),
      key: "key1".to_string(),
      value: Some("value1".to_string()),
      has_count: false,
      i18next_options: None,
    };
    let mut unique_count = HashMap::new();
    let mut unique_plurals_count = HashMap::new();
    let value = Value::Object(Default::default());
    let options = Default::default();

    let result = transform_entry(&entry, &mut unique_count, &mut unique_plurals_count, &value, &options, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({"default": {"key1": "value1"}}));
    assert_eq!(unique_count.get("default"), Some(&1));
    assert_eq!(unique_plurals_count.get("default"), Some(&0));
  }
}
