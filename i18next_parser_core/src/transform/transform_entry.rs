use color_eyre::owo_colors::OwoColorize;
use log::warn;

use crate::{
  Entry,
  config::Config,
  helper::{dot_path_to_hash::dot_path_to_hash, get_char_diff::get_char_diff},
  merger::merge_all_values::FoundValue,
  models::Conflict,
};

/// Transforms an entry into a JSON object.
pub fn transform_entry(entry: &Entry, options: &Config, locale: &str, found_values: &mut FoundValue) {
  let namespace = if let Some(ns) = &entry.namespace {
    ns
  } else {
    &options.default_namespace
  };

  let values = dot_path_to_hash(entry, locale, options, found_values);

  if let Some(values) = &values {
    for (key, (value, conflict)) in values.iter() {
      found_values.insert(key.to_string(), value.clone());

      if let Some(Conflict::Value(old, new)) = conflict {
        let separator: &str = options.namespace_separator.as_ref();
        let diff = get_char_diff(&old.value, &new.value);
        if options.verbose {
          old.location.print();
          new.location.print();
        }
        if options.fail_on_warnings {
          panic!(
            "Found translation key already mapped to a map or parent of new key already mapped to a string: {key}",
            key = format!(
              "{namespace}{separator}{key}",
              namespace = namespace.bright_yellow(),
              key = entry.key.blue()
            )
            .italic(),
          )
        }

        warn!(
          "Found same keys with different values: {key}: {diff}",
          key = format!(
            "{namespace}{separator}{key}",
            namespace = namespace.bright_yellow(),
            key = entry.key.blue()
          )
          .italic(),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn test_transform_entry() {
    let entry = Entry {
      location: Default::default(),
      namespace: Some("default".to_string()),
      key: "key1".to_string(),
      value: Some("value1".to_string()),
      has_count: false,
      i18next_options: None,
      context: None,
    };
    let mut value = FoundValue::new();
    let options = Default::default();

    transform_entry(&entry, &options, "en", &mut value);

    assert_eq!(value, HashMap::from([("default.key1".to_string(), "value1".into())]));
  }
}
