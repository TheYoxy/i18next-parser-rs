//! Module containing the dot_path_to_hash function.

use std::collections::HashMap;

use color_eyre::owo_colors::OwoColorize;
use log::trace;

use crate::{
  merger::merge_all_values::{FoundEntry, FoundValue},
  Config,
  Entry,
  Location,
};

/// Enum representing the type of conflict that can occur when converting a dot path to a hash.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Conflict {
  Value(ConflictEntry, ConflictEntry),
}

/// Reprensents a conflict entry.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConflictEntry {
  /// The value that conflicts.
  pub value: String,
  /// The location of the conflict.
  pub location: Location,
}
impl From<&str> for ConflictEntry {
  fn from(value: &str) -> Self {
    Self { value: value.to_string(), location: Location::default() }
  }
}
impl From<String> for ConflictEntry {
  fn from(value: String) -> Self {
    Self { value, location: Location::default() }
  }
}

impl ConflictEntry {
  /// Default constructor for ConflictEntry.
  pub fn new(key: String, location: Location) -> Self {
    Self { value: key, location }
  }
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
pub fn dot_path_to_hash(
  entry: &Entry,
  suffix: Option<&str>,
  config: &Config,
  found_value: &FoundValue,
) -> Option<HashMap<String, (FoundEntry, Option<Conflict>)>> {
  let separator = &config.key_separator;
  let context_separator = &config.context_separator;

  if entry.key.is_empty() {
    return None;
  }

  let entry_path = {
    let base_path = entry
      .namespace
      .as_ref()
      .or(Some(&config.default_namespace))
      .map(|ns| format!("{ns}{separator}{key}", key = entry.key))
      .unwrap();

    let mut path =
      base_path.replace(r#"\\n"#, "\\n").replace(r#"\\r"#, "\\r").replace(r#"\\t"#, "\\t").replace(r#"\\\\"#, "\\");

    if let Some(suffix) = suffix {
      path += suffix;
    }

    trace!("Path: {:?}", path.purple());
    if path.ends_with(separator) {
      trace!("Removing trailing separator from path: {:?}", path.purple());
      path = path[..path.len() - separator.len()].into();
      trace!("New path: {:?}", path.purple());
    }

    path
  };

  trace!("Val {:?} {:?}", entry.key.purple(), entry.value.cyan());
  let mut new_values = HashMap::new();

  if let Some(context_value) = &entry.context {
    for context in context_value {
      merge_values(
        entry,
        config,
        found_value,
        format!("{entry_path}{context_separator}{context}"),
        &mut new_values,
        true,
      );
    }
  } else {
    merge_values(entry, config, found_value, entry_path, &mut new_values, false);
  }

  Some(new_values)
}

fn merge_values(
  entry: &Entry,
  config: &Config,
  found_value: &HashMap<String, FoundEntry>,
  entry_path: String,
  new_values: &mut HashMap<String, (FoundEntry, Option<Conflict>)>,
  has_context: bool,
) {
  let old_value = found_value.get(&entry_path);
  let (new_value, conflict): (&str, Option<Conflict>) = entry
    .value
    .as_ref()
    .map(|new_value| {
      if let Some(old_value) = old_value {
        let old_location = &old_value.location;
        let old_value = &old_value.value;
        trace!("Values {:?} -> {:?}", old_value.purple(), new_value.purple());
        if *old_value != *new_value && !old_value.is_empty() {
          if new_value.is_empty() {
            trace!("new value is empty, keeping old value {old_value:?}");
            (old_value.as_str(), None)
          } else if has_context {
            trace!("old value is different from new value, but has context. Keeping old value {old_value:?}");
            // Since there is a context, we don't update the old value
            (old_value.as_str(), None)
          } else {
            // We are free to update the old value
            (
              new_value.as_str(),
              Some(Conflict::Value(
                ConflictEntry::new(old_value.clone(), old_location.clone()),
                ConflictEntry::new(new_value.clone(), Location {
                  start: entry.location.start,
                  end: entry.location.end,
                  file: entry.location.file.clone(),
                }),
              )),
            )
          }
        } else {
          trace!("Old value is empty or match new value, assigning new value {:?}", new_value.purple());
          (new_value.as_str(), None)
        }
      } else {
        trace!("No old value, assigning new value {:?}", new_value.purple());
        (new_value.as_str(), None)
      }
    })
    .map(|(v, conflict)| (v.trim(), conflict))
    .unwrap_or_default();

  if let Some(namespace) = &entry.namespace {
    trace!("Setting [{:?}] {:?} -> {:?}", namespace.cyan(), entry_path.yellow(), new_value.purple());
  } else {
    log::info!(
      "Setting to default namespace [{:?}] {:?} -> {:?}",
      config.default_namespace.cyan(),
      entry_path.yellow(),
      new_value.purple()
    );
  };
  new_values.insert(
    entry_path,
    (FoundEntry { value: new_value.to_string(), location: Location { ..entry.location.clone() } }, conflict),
  );
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn handles_empty_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("".into()),
      key: "".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
      ..Default::default()
    };
    let target = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &target);
    assert!(result.is_none());
  }

  #[test]
  fn handles_nonexistent_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("nonexistent".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
      ..Default::default()
    };
    let target = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("nonexistent.key"));
    assert_eq!(result.get("nonexistent.key").unwrap().0.value, "default_value");
    assert_eq!(result.get("nonexistent.key").unwrap().1, None);
  }

  #[test]
  fn handles_existing_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
      ..Default::default()
    };

    let mut target = FoundValue::new();
    target.insert("namespace.key".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key"));
    assert_eq!(result.get("namespace.key").unwrap().0.value, "default_value");
    assert_eq!(
      result.get("namespace.key").unwrap().1,
      Some(Conflict::Value("existing_value".into(), "default_value".into()))
    );
  }

  #[test]
  fn handle_add_entries() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key2".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
      ..Default::default()
    };

    let mut target = FoundValue::new();
    target.insert("namespace.key1".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(!result.contains_key("namespace.key1"));
    assert!(result.contains_key("namespace.key2"));
    assert_eq!(result.get("namespace.key2").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key2").unwrap().1, None);
  }

  #[test]
  fn handles_suffix() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
      ..Default::default()
    };

    let mut value = FoundValue::new();
    value.insert("namespace.key_suffix".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, Some("_suffix"), &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key_suffix"));
    assert_eq!(result.get("namespace.key_suffix").unwrap().0.value, "default_value");
    assert_eq!(
      result.get("namespace.key_suffix").unwrap().1,
      Some(Conflict::Value("existing_value".into(), "default_value".into()))
    );
  }

  #[test]
  fn handle_context() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      context: Some(vec!["context1".into(), "context2".into()]),
      ..Default::default()
    };

    let value = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key_context1"));
    assert_eq!(result.get("namespace.key_context1").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_context1").unwrap().1, None);

    assert!(result.contains_key("namespace.key_context2"));
    assert_eq!(result.get("namespace.key_context2").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_context2").unwrap().1, None);
  }

  #[test]
  fn do_not_override_existing_context_value() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      context: Some(vec!["context1".into(), "context2".into()]),
      ..Default::default()
    };

    let mut value = FoundValue::new();
    value.insert("namespace.key_context1".into(), "existing_value".into());
    value.insert("namespace.key_context2".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, None, &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key_context1"));
    assert_eq!(result.get("namespace.key_context1").unwrap().0.value, "existing_value");
    assert_eq!(result.get("namespace.key_context1").unwrap().1, None);

    assert!(result.contains_key("namespace.key_context2"));
    assert_eq!(result.get("namespace.key_context2").unwrap().0.value, "existing_value");
    assert_eq!(result.get("namespace.key_context2").unwrap().1, None);
  }
}
