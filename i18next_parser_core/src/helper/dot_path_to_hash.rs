//! Module containing the dot_path_to_hash function.

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
  found_value: &mut FoundValue,
) -> Option<Conflict> {
  let separator = &config.key_separator;

  if entry.key.is_empty() {
    return None;
  }

  let entry_path = {
    let base_path = entry
      .namespace
      .clone()
      .or(Some(config.default_namespace.clone()))
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

  let segments: Vec<&str> = entry_path.split(separator).collect();
  trace!("Val {:?} {:?}", entry.key.purple(), entry.value.cyan());
  let mut conflict: Option<Conflict> = None;

  let old_value = lookup_by_key(found_value, &segments);

  let new_value: String = entry
    .value
    .clone()
    .map(|new_value| {
      if let Some(old_value) = old_value {
        let old_location = &old_value.location;
        let old_value = &old_value.value;
        trace!("Values {:?} -> {:?}", old_value.purple(), new_value.purple());
        if *old_value != new_value && !old_value.is_empty() {
          if new_value.is_empty() {
            trace!("new value is empty, keeping old value {old_value:?}");
            old_value.clone()
          } else {
            // log::warn!(
            //   "Conflict: {:?} -> {:?} -> {:?}",
            //   path.yellow().italic(),
            //   old_value.purple().italic(),
            //   new_value.purple()
            // );
            conflict = Some(Conflict::Value(
              ConflictEntry::new(old_value.clone(), old_location.clone()),
              ConflictEntry::new(new_value.clone(), Location {
                start: entry.location.start,
                end: entry.location.end,
                file: entry.location.file.clone(),
              }),
            ));
            new_value
          }
        } else {
          trace!("Old value is empty or match new value, assigning new value {:?}", new_value.purple());
          new_value
        }
      } else {
        trace!("No old value, assigning new value {:?}", new_value.purple());
        new_value
      }
    })
    .map(|v| v.trim().into())
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
  found_value.insert(entry_path, FoundEntry { value: new_value, location: Location { ..entry.location.clone() } });

  conflict
}

/// Lookup a value in a JSON object by key.
///
/// # Arguments
///
/// * `segments`: The segments of the key.
///
/// returns: (`Option<String>`, `Option<Conflict>`, &'a mut Value, &'a str) - A tuple containing a mutable reference to the value and an optional conflict.
#[inline]
fn lookup_by_key<'a>(found_value: &'a FoundValue, segments: &'a [&'a str]) -> Option<&'a FoundEntry> {
  let old_value = found_value.get(&segments.join("."));
  old_value
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;

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
      location: Default::default(),
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

    let result = dot_path_to_hash(&entry, &mut target, None, &config);

    assert_eq!(
      *result.target,
      json!({
        "namespace": {
          "key": "default_value"
        }
      })
    );

    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
  }

  #[test]
  fn handles_empty_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("".into()),
      key: "".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      has_count: true,
    };
    let mut target = json!({});
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &mut target, None, &config);

    assert_eq!(*result.target, json!({}));
    assert!(result.conflict.is_none());
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
    };
    let mut target = json!({});
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &mut target, None, &config);

    assert_eq!(
      *result.target,
      json!({
          "nonexistent": {
              "key": "default_value"
          }
      })
    );
    assert!(result.conflict.is_none());
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
    };
    let mut target = json!({
        "namespace": {
            "key": "existing_value"
        }
    });
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &mut target, None, &config);

    assert_eq!(
      *result.target,
      json!({
          "namespace": {
              "key": "default_value"
          }
      })
    );
    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
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
    };
    let mut target = json!({
        "namespace": {
            "key1": "default_value"
        }
    });
    let config = Default::default();

    let result = dot_path_to_hash(&entry, &mut target, None, &config);

    assert_eq!(
      *result.target,
      json!({
          "namespace": {
              "key1": "default_value",
              "key2": "default_value"
          }
      })
    );
    assert_eq!(result.conflict, None);
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
    };
    let mut target = json!({
        "namespace": {
            "key_suffix": "existing_value"
        }
    });
    let mut value = FoundValue::new();
    value.insert("namespace.key_suffix".into(), FoundEntry {
      value: "existing_value".into(),
      location: Default::default(),
    });
    let config = Default::default();

    let result = dot_path_to_hash(&entry, Some("_suffix"), &config, &mut value);
    assert_eq!(
      result,
      Some(Conflict::Value(
        ConflictEntry::new("existing_value".into(), Default::default()),
        ConflictEntry::new("default_value".into(), Default::default())
      ))
    );
  }
}
