//! Module containing the dot_path_to_hash function.
use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use serde_json::{Map, Value};

use crate::{helper::skip_last::SkipLast, Config, Entry};

/// Enum representing the type of conflict that can occur when converting a dot path to a hash.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Conflict {
  Key(String),
  Value(String, String),
}

/// Struct representing the result of converting a dot path to a hash.
#[derive(Debug)]
pub struct DotPathToHashResult<'a> {
  pub target: &'a Value,
  pub conflict: Option<Conflict>,
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
pub fn dot_path_to_hash<'a>(
  entry: &Entry,
  target: &'a mut Value,
  suffix: Option<&str>,
  config: &Config,
) -> DotPathToHashResult<'a> {
  let separator = &config.key_separator;

  if entry.key.is_empty() {
    return DotPathToHashResult { target, conflict: None };
  }

  let path = {
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

  let segments: Vec<&str> = path.split(separator).collect();
  trace!("Val {:?} {:?} {:?}", &target.yellow(), entry.key.purple(), entry.value.cyan());

  let (old_value, mut conflict, inner, last_segment) = lookup_by_key(target, &segments);

  let new_value: String = entry
    .value
    .clone()
    .map(|new_value| {
      if let Some(old_value) = old_value {
        trace!("Values {:?} -> {:?}", old_value.purple(), new_value.purple());
        if old_value != new_value && !old_value.is_empty() {
          if new_value.is_empty() {
            trace!("new value is empty, keeping old value {old_value:?}");
            old_value
          } else {
            warn!("Conflict: {:?} -> {:?} -> {:?}", path.yellow().italic(), old_value.purple(), new_value.purple());
            conflict = Some(Conflict::Value(old_value, new_value.clone()));
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
    debug!("Setting [{:?}] {:?} -> {:?}", namespace.cyan(), path.yellow(), new_value.purple());
  } else {
    debug!("Setting {:?} -> {:?}", path.yellow(), new_value.purple());
  };
  inner[last_segment] = Value::String(new_value);

  DotPathToHashResult { target, conflict }
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
      if inner[segment].is_string() {
        conflict = Some(Conflict::Key(segment.to_string()));
      }
      if inner[segment].is_null() || conflict.is_some() {
        inner[segment] = Value::Object(Map::new());
      }

      &mut inner[segment]
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

    let result = dot_path_to_hash(&entry, &mut target, Some("_suffix"), &config);

    assert_eq!(
      *result.target,
      json!({
          "namespace": {
              "key_suffix": "default_value"
          }
      })
    );
    assert_eq!(result.conflict, Some(Conflict::Value("existing_value".into(), "default_value".into())));
  }
}
