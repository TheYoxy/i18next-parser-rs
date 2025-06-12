//! Module containing the dot_path_to_hash function.

use std::collections::HashMap;

use color_eyre::owo_colors::OwoColorize;
use log::trace;

use crate::{
  merger::{merge_all_values::FoundValue, FoundEntry},
  transform::plural::{I18NVersion, PluralResolver},
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
/// * `locale` - An optional reference to a string representing the suffix to be added to the path.
/// * `config` - A reference to a Config object.
/// * `found_value` - A reference to a FoundValue object containing previously found values.
///
/// # Returns
///
/// * A DotPathToHashResult object.
pub fn dot_path_to_hash(
  entry: &Entry,
  locale: &str,
  config: &Config,
  found_value: &FoundValue,
) -> Option<HashMap<String, (FoundEntry, Option<Conflict>)>> {
  if entry.key.is_empty() {
    return None;
  }

  let separator = &config.key_separator;
  let entry_path = {
    let base_path = entry
      .namespace
      .as_ref()
      .or(Some(&config.default_namespace))
      .map(|ns| format!("{ns}{separator}{key}", key = entry.key))
      .unwrap();

    let mut path =
      base_path.replace(r#"\\n"#, "\\n").replace(r#"\\r"#, "\\r").replace(r#"\\t"#, "\\t").replace(r#"\\\\"#, "\\");

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

  let plural_resolver = PluralResolver::new(false, &config.plural_separator, I18NVersion::V4);
  let context_suffixes = entry.context.as_ref().map(|context| {
    let context_separator = &config.context_separator;
    context.iter().map(|context| format!("{context_separator}{context}")).collect::<Vec<_>>()
  });
  let count_suffixes = if entry.has_count {
    plural_resolver
      .get_suffixes(locale)
      .inspect_err(|e| {
        if config.fail_on_warnings {
          panic!("Error getting suffixes for locale {entry_path}: {e}");
        }
        log::error!("Error getting suffixes: {e}");
      })
      .ok()
  } else {
    None
  };

  trace!("Context entries: {context_suffixes:?}", context_suffixes = context_suffixes.magenta());
  trace!("Count entries: {count_suffixes:?}", count_suffixes = count_suffixes.magenta());
  match (&context_suffixes, &count_suffixes) {
    (Some(context_suffixes), Some(count_suffixes)) => {
      for context in context_suffixes {
        for count in count_suffixes {
          let full_path = format!("{entry_path}{context}{count}");
          merge_values(entry, config, found_value, full_path, &mut new_values, true);
        }
      }
    },
    (Some(context_suffixes), None) => {
      for context in context_suffixes {
        let full_path = format!("{entry_path}{context}");
        merge_values(entry, config, found_value, full_path, &mut new_values, true);
      }
    },

    (None, Some(count_suffixes)) => {
      for count in count_suffixes {
        let full_path = format!("{entry_path}{count}");
        merge_values(entry, config, found_value, full_path, &mut new_values, true);
      }
    },
    (None, None) => {
      merge_values(entry, config, found_value, entry_path, &mut new_values, false);
    },
  };

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

  new_values.insert(entry_path, (FoundEntry::new_with_location(new_value, entry.location.clone()), conflict));
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::*;

  #[test_log::test]
  fn handles_empty_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("".into()),
      key: "".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      ..Default::default()
    };
    let target = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &target);
    assert!(result.is_none());
  }

  #[test_log::test]
  fn handles_nonexistent_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("nonexistent".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      ..Default::default()
    };
    let target = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("nonexistent.key"));
    assert_eq!(result.get("nonexistent.key").unwrap().0.value, "default_value");
    assert_eq!(result.get("nonexistent.key").unwrap().1, None);
  }

  #[test_log::test]
  fn handles_existing_path() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      ..Default::default()
    };

    let mut target = FoundValue::new();
    target.insert("namespace.key".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key"));
    assert_eq!(result.get("namespace.key").unwrap().0.value, "default_value");
    assert_eq!(
      result.get("namespace.key").unwrap().1,
      Some(Conflict::Value("existing_value".into(), "default_value".into()))
    );
  }

  #[test_log::test]
  fn handle_add_entries() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key2".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      ..Default::default()
    };

    let mut target = FoundValue::new();
    target.insert("namespace.key1".into(), "existing_value".into());
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &target);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(!result.contains_key("namespace.key1"));
    assert!(result.contains_key("namespace.key2"));
    assert_eq!(result.get("namespace.key2").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key2").unwrap().1, None);
  }

  #[test_log::test]
  fn handle_count() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      context: None,
      has_count: true,
    };

    let value = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key_one"));
    assert_eq!(result.get("namespace.key_one").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_one").unwrap().1, None);

    assert!(result.contains_key("namespace.key_other"));
    assert_eq!(result.get("namespace.key_other").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_other").unwrap().1, None);
  }

  #[test_log::test]
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

    let result = dot_path_to_hash(&entry, "en", &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert!(result.contains_key("namespace.key_context1"));
    assert_eq!(result.get("namespace.key_context1").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_context1").unwrap().1, None);

    assert!(result.contains_key("namespace.key_context2"));
    assert_eq!(result.get("namespace.key_context2").unwrap().0.value, "default_value");
    assert_eq!(result.get("namespace.key_context2").unwrap().1, None);
  }

  #[test_log::test]
  fn handle_context_and_count() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("namespace".into()),
      key: "key".into(),
      value: Some("default_value".into()),
      i18next_options: None,
      context: Some(vec!["context1".into(), "context2".into()]),
      has_count: true,
    };

    let value = FoundValue::new();
    let config = Default::default();

    let result = dot_path_to_hash(&entry, "en", &config, &value);
    assert!(result.is_some());
    let result = result.unwrap();

    assert_eq!(result.get("namespace.key_context1_one"), Some(&(FoundEntry::new("default_value"), None)));
    assert_eq!(result.get("namespace.key_context2_one"), Some(&(FoundEntry::new("default_value"), None)));
    assert_eq!(result.get("namespace.key_context1_other"), Some(&(FoundEntry::new("default_value"), None)));
    assert_eq!(result.get("namespace.key_context2_other"), Some(&(FoundEntry::new("default_value"), None)));
  }

  #[test_log::test]
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

    let result = dot_path_to_hash(&entry, "en", &config, &value);
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
