use log::{trace, warn};
use serde_json::{Map, Value};

use crate::config::Config;
use crate::visitor::Entry;

/// Enum representing the type of conflict that can occur when converting a dot path to a hash.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum Conflict {
  Key,
  Value(String, String),
}

/// Struct representing the result of converting a dot path to a hash.
#[derive(Debug)]
pub(crate) struct DotPathToHashResult {
  pub(crate) target: Value,
  pub(crate) duplicate: bool,
  pub(crate) conflict: Option<Conflict>,
}

/// Converts an entry with a dot path to a hash.
///
/// # Arguments
///
/// * `entry` - A reference to an Entry object.
/// * `target` - A reference to a Value object representing the target JSON.
/// * `suffix` - An optional reference to a string representing the suffix to be added to the path.
/// * `config` - A reference to a Config object.
///
/// # Returns
///
/// * A DotPathToHashResult object.
pub(crate) fn dot_path_to_hash(
  entry: &Entry,
  target: &Value,
  suffix: Option<&str>,
  config: &Config,
) -> DotPathToHashResult {
  let mut target = target.clone();
  let separator = config.key_separator.clone().unwrap_or(".".to_string());

  if entry.key.is_empty() {
    return DotPathToHashResult { target, duplicate: false, conflict: None };
  }

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
  trace!("Val {:?} {:?} {:?}", &target, entry.key, entry.value);

  let mut inner = &mut target;
  let mut conflict: Option<Conflict> = None;
  #[allow(clippy::needless_range_loop)]
  for i in 0..segments.len() - 1 {
    let segment = segments[i];
    if !segment.is_empty() {
      if inner[segment].is_string() {
        conflict = Some(Conflict::Key);
      }
      if inner[segment].is_null() || conflict.is_some() {
        inner[segment] = Value::Object(Map::new());
      }
      inner = &mut inner[segment];
    }
  }
  let mut duplicate = false;

  let last_segment = segments[segments.len() - 1];
  let old_value = inner[last_segment].as_str().map(|s| s.to_owned());
  let mut has_warn = false;
  let new_value = entry
    .value
    .clone()
    .map(|new_value| {
      if let Some(old_value) = old_value {
        if old_value != new_value {
          warn!("Values {:?} -> {:?}", old_value, new_value);
        }
        trace!("Values {:?} -> {:?}", old_value, new_value);
        if old_value != new_value && !old_value.is_empty() {
          if new_value.is_empty() {
            old_value
          } else {
            conflict = Some(Conflict::Value(old_value, new_value.clone()));
            duplicate = true;
            has_warn = true;

            new_value
          }
        } else {
          new_value
        }
      } else {
        new_value
      }
    })
    .map(|v| v.trim().to_string())
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
    trace!("Setting {path:?} -> {new_value:?}");
    inner[last_segment] = Value::String(new_value);
  }

  DotPathToHashResult { target: target.clone(), duplicate, conflict }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_dot_path_to_hash() {
    let entry = Entry {
      namespace: Some("namespace".to_string()),
      key: "key".to_string(),
      value: Some("default_value".to_string()),
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
    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".to_string(), "default_value".to_string())));
  }

  #[test]
  fn dot_path_to_hash_handles_empty_path() {
    let entry = Entry {
      namespace: Some("".to_string()),
      key: "".to_string(),
      value: Some("default_value".to_string()),
      i18next_options: None,
      count: None,
    };
    let target = json!({});
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &target, None, &config);

    assert_eq!(result.target, json!({}));
    assert!(!result.duplicate);
    assert!(result.conflict.is_none());
  }

  #[test]
  fn dot_path_to_hash_handles_nonexistent_path() {
    let entry = Entry {
      namespace: Some("nonexistent".to_string()),
      key: "key".to_string(),
      value: Some("default_value".to_string()),
      i18next_options: None,
      count: None,
    };
    let target = json!({});
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &target, None, &config);

    assert_eq!(
      result.target,
      json!({
          "nonexistent": {
              "key": "default_value"
          }
      })
    );
    assert!(!result.duplicate);
    assert!(result.conflict.is_none());
  }

  #[test]
  fn dot_path_to_hash_handles_existing_path() {
    let entry = Entry {
      namespace: Some("namespace".to_string()),
      key: "key".to_string(),
      value: Some("default_value".to_string()),
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
    assert!(result.duplicate);
    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".to_string(), "default_value".to_string())));
  }

  #[test]
  fn dot_path_to_hash_handles_suffix() {
    let entry = Entry {
      namespace: Some("namespace".to_string()),
      key: "key".to_string(),
      value: Some("default_value".to_string()),
      i18next_options: None,
      count: None,
    };
    let target = json!({
        "namespace": {
            "key_suffix": "existing_value"
        }
    });
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &target, Some("_suffix"), &config);

    assert_eq!(
      result.target,
      json!({
          "namespace": {
              "key_suffix": "default_value"
          }
      })
    );
    assert!(result.duplicate);
    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".to_string(), "default_value".to_string())));
  }
}
