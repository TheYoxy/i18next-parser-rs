//! This module contains the logic to transform entries into a JSON object.
use std::collections::HashMap;

use color_eyre::{eyre::bail, owo_colors::OwoColorize};
use log::{debug, error, trace, warn};
use serde_json::{Map, Value};

use crate::{
  config::Config,
  helper::{get_char_diff::get_char_diff, skip_last::SkipLast},
  transform::plural::PluralResolver,
  Entry,
};

/// Represents the result of transforming entries.
pub struct TransformEntriesResult {
  /// The unique count of entries.
  pub unique_count: HashMap<String, usize>,
  /// The unique count of plural entries.
  pub unique_plurals_count: HashMap<String, usize>,
  /// The transformed value.
  pub value: Value,
  /// The locale of the transformed value.
  pub locale: String,
}

type Count = HashMap<String, usize>;

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
#[tracing::instrument(skip_all, err, target = "instrument")]
pub fn transform_entries(
  entries: &[Entry],
  locale: &str,
  config: &Config,
) -> color_eyre::Result<TransformEntriesResult> {
  let mut value = Value::Object(Default::default());
  let mut unique_count = Count::new();
  let mut unique_plurals_count = Count::new();

  for entry in entries {
    if entry.has_count {
      match PluralResolver::default().get_suffixes(locale) {
        Ok(suffixes) => {
          for suffix in suffixes {
            transform_entry(&mut value, entry, &mut unique_count, &mut unique_plurals_count, config, Some(&suffix))?;
          }
        },
        Err(e) => {
          error!("Error getting suffixes: {}", e);
        },
      }
    } else {
      transform_entry(&mut value, entry, &mut unique_count, &mut unique_plurals_count, config, None)?;
    }
  }

  Ok(TransformEntriesResult { unique_count, unique_plurals_count, value, locale: locale.to_string() })
}

/// Transforms an entry into a JSON object.
pub fn transform_entry(
  value: &mut Value,
  entry: &Entry,
  unique_count: &mut Count,
  unique_plurals_count: &mut Count,
  options: &Config,
  suffix: Option<&str>,
) -> color_eyre::Result<()> {
  let namespace = entry.namespace.clone().unwrap_or("default".to_string());
  if !unique_count.contains_key(&namespace) {
    unique_count.insert(namespace.clone(), 0);
  }
  if !unique_plurals_count.contains_key(&namespace) {
    unique_plurals_count.insert(namespace.clone(), 0);
  }

  let conflict = dot_path_to_hash(value, entry, suffix, options);

  match conflict {
    Some(Conflict::Key(key, value)) => {
      if options.fail_on_warnings {
        bail!("Found translation key already mapped to a map or parent of new key already mapped to a string: {key} [{value}]")
      } else {
        warn!("Found translation key already mapped to a map or parent of new key already mapped to a string: {key} [{value}]");
      }
    },
    Some(Conflict::Value(old, new)) => {
      let separator: &str = options.namespace_separator.as_ref();
      let diff = get_char_diff(&old, &new);
      if options.fail_on_warnings {
        bail!(
          "Found same keys with different values: {namespace}{separator}{key}: {diff}",
          namespace = namespace.bright_yellow(),
          key = entry.key.blue(),
          diff = diff
        );
      } else {
        warn!(
          "Found same keys with different values: {namespace}{separator}{key}: {diff}",
          namespace = namespace.bright_yellow(),
          key = entry.key.blue(),
          diff = diff
        );
      }
    },
    _ => {
      *unique_count.get_mut(&namespace).unwrap() += 1;
      if suffix.is_some() {
        *unique_plurals_count.get_mut(&namespace).unwrap() += 1;
      }
    },
  }

  Ok(())
}

/// Enum representing the type of conflict that can occur when converting a dot path to a hash.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Conflict {
  Key(String, String),
  Value(String, String),
}

/// Converts an entry with a dot path to a hash.
///
/// # Arguments
/// l
/// * `entry` - A reference to an Entry object.
/// * `target` - A reference to a Value object representing the target JSON.
/// * `suffix` - An optional reference to a string representing the suffix to be added to the path.
/// * `config` - A reference to a Config object.
///
/// # Returns
///
/// * A DotPathToHashResult object.
pub fn dot_path_to_hash<'a>(
  target: &'a mut Value,
  entry: &Entry,
  suffix: Option<&str>,
  config: &Config,
) -> Option<Conflict> {
  let separator = &config.key_separator;

  if entry.key.is_empty() {
    return None;
  }

  let path = {
    let ns = match entry.namespace {
      Some(ref ns) => ns,
      None => &config.default_namespace,
    };
    let path = format!("{ns}{separator}{key}", key = entry.key)
      .replace(r#"\\n"#, "\\n")
      .replace(r#"\\r"#, "\\r")
      .replace(r#"\\t"#, "\\t")
      .replace(r#"\\\\"#, "\\");

    let path = if let Some(suffix) = suffix { path + suffix } else { path };
    trace!("Path: {:?}", path.purple());

    if path.ends_with(separator) {
      trace!("Removing trailing separator from path: {:?}", path.purple());
      let path = path[..path.len() - separator.len()].to_string();
      trace!("New path: {:?}", path.purple());
      path
    } else {
      path
    }
  };

  let segments = path.split(separator).collect::<Vec<&str>>();
  trace!("Val {:?} {:?} {:?}", &target.yellow(), entry.key.purple(), entry.value.cyan());

  let (old_value, mut conflict, inner, last_segment) = lookup_by_key(target, &segments);

  let new_value: String = match (&entry.value, old_value) {
    (Some(new_value), Some(old_value)) if *old_value != *new_value && !old_value.is_empty() => {
      if new_value.is_empty() {
        trace!("new value is empty, keeping old value {old_value:?}");
        old_value
      } else {
        warn!("Conflict: {:?} -> {:?} -> {:?}", path.yellow().italic(), old_value.purple(), new_value.purple());
        conflict = Some(Conflict::Value(old_value, new_value.clone()));
        new_value.clone()
      }
    },
    (Some(new_value), Some(_)) => {
      trace!("Old value is empty or match new value, assigning new value {:?}", new_value.purple());
      new_value.clone()
    },
    (Some(new_value), None) => {
      trace!("No old value, assigning new value {:?}", new_value.purple());
      new_value.clone()
    },
    (None, _) => {
      warn!("No value provided for key {:?} in namespace {:?}, skipping", entry.key.yellow(), entry.namespace.yellow());
      Default::default()
    },
  };

  if let Some(namespace) = &entry.namespace {
    debug!("Setting [{:?}] {:?} -> {:?}", namespace.cyan(), path.yellow(), new_value.purple());
  } else {
    debug!("Setting {:?} -> {:?}", path.yellow(), new_value.purple());
  };

  inner[last_segment] = Value::String(new_value);

  conflict
}

/// Lookup a value in a JSON object by key.
///
/// # Arguments
///
/// * `target`: The target JSON object.
/// * `segments`: The segments of the key.
///
/// returns: (`Option<String>`, `Option<Conflict>`, &'a mut Value, &'a str) - A tuple containing a mutable reference to the value and an optional conflict.
fn lookup_by_key<'a>(
  target: &'a mut Value,
  segments: &'a [&'a str],
) -> (Option<String>, Option<Conflict>, &'a mut Value, &'a str) {
  let mut conflict: Option<Conflict> = None;

  let inner = segments.iter().skip_last().fold(target, |inner, segment| {
    if !segment.is_empty() {
      match inner {
        Value::String(value) => {
          conflict = Some(Conflict::Key(segment.to_string(), value.clone()));
          &mut inner[segment]
        },
        Value::Null => {
          inner[segment] = Value::Object(Map::new());
          &mut inner[segment]
        },
        _ => &mut inner[segment],
      }
    } else {
      inner
    }
  });

  let last_segment = segments[segments.len() - 1];
  let old_value = inner[last_segment].as_str().map(|s| s.to_owned());
  (old_value, conflict, inner, last_segment)
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;
  use crate::Entry;

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
  fn test_transform_entries_complex() {
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
        namespace: Some("default".to_string()),
        key: "key3.key1".to_string(),
        has_count: false,
        value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key3.key2".to_string(),
        has_count: false,
        value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key3.key3.key1".to_string(),
        has_count: false,
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

    assert_eq!(result.unique_count.get("default"), Some(&6));
    assert_eq!(result.unique_count.get("custom"), Some(&1));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("custom"), Some(&0));
    assert_eq!(result.value, json!({
        "default": {
          "key1": "value1",
          "key2_one": "value2",
          "key2_other": "value2",
          "key3": {
            "key1": "value2",
            "key2": "value2",
            "key3": {
              "key1": "value2"
            }
          }
        },
        "custom": {
          "key3": "value3",
        }
      })
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
    let mut value = Value::Object(Default::default());
    let options = Default::default();

    let result = transform_entry(&mut value, &entry, &mut unique_count, &mut unique_plurals_count, &options, None);

    assert!(result.is_ok());
    assert_eq!(value, json!({"default": {"key1": "value1"}}));
    assert_eq!(unique_count.get("default"), Some(&1));
    assert_eq!(unique_plurals_count.get("default"), Some(&0));
  }

  #[test]
  fn test_lookup_by_key() {
    let mut target = json!({ "a": { "b": { "c": "value" } } });
    let entry = vec!["a", "b", "c"];

    {
      let (value, conflict, obj, key) = lookup_by_key(&mut target, &entry);

      assert_eq!(value, Some("value".into()));
      assert_eq!(conflict, None);
      assert_eq!(obj, &json!({ "c": "value" }));
      assert_eq!(key, "c");
      obj[key] = Value::String("new_value".into());
    }

    // validate that the obj returned is from the same instance of the object
    let target = target.get("a").unwrap().get("b").unwrap().get("c").unwrap();
    assert_eq!(*target, Value::String("new_value".into()));
  }

  #[test]
  fn base() {
    let entry = Entry {
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({
      "namespace": {
        "key": "existing_value"
      }
    });
    let config = Default::default();

    let conflict = dot_path_to_hash(&mut target, &entry, None, &config);

    assert_eq!(target, json!({"namespace": {"key": "default_value"}}));
    assert_eq!(conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
  }

  #[test]
  fn handles_empty_path() {
    let entry = Entry {
      namespace: Some("".into()),
      key: "".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({});
    let config = Default::default();
    let conflict = dot_path_to_hash(&mut target, &entry, None, &config);

    assert_eq!(target, json!({}));
    assert!(conflict.is_none());
  }

  #[test]
  fn handles_nonexistent_path() {
    let entry = Entry {
      namespace: Some("nonexistent".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({});
    let config = Default::default();

    let conflict = dot_path_to_hash(&mut target, &entry, None, &config);

    assert_eq!(
      target,
      json!({
          "nonexistent": {
              "key": "default_value"
          }
      })
    );
    assert!(conflict.is_none());
  }

  #[test]
  fn handles_existing_path() {
    let entry = Entry {
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({
        "namespace": {
            "key": "existing_value"
        }
    });
    let config = Default::default();

    let conflict = dot_path_to_hash(&mut target, &entry, None, &config);

    assert_eq!(target, json!({"namespace": {"key": "default_value"}}));
    assert_eq!(conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
  }

  #[test]
  fn handle_add_entries() {
    let entry = Entry {
      namespace: Some("namespace".into()),
      key: "key2".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({
        "namespace": {
            "key1": "default_value"
        }
    });
    let config = Default::default();
    let conflict = dot_path_to_hash(&mut target, &entry, None, &config);

    assert_eq!(target, json!({"namespace": {"key1": "default_value","key2": "default_value"}}));
    assert_eq!(conflict, None);
  }

  #[test]
  fn handles_suffix() {
    let entry = Entry {
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({
        "namespace": {
            "key_suffix": "existing_value"
        }
    });
    let config = Default::default();
    let conflict = dot_path_to_hash(&mut target, &entry, Some("_suffix"), &config);

    assert_eq!(
      target,
      json!({
          "namespace": {
              "key_suffix": "default_value"
          }
      })
    );
    assert_eq!(conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
  }
}
