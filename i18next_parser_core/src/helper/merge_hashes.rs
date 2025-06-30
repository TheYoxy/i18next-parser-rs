//! This module contains the merge_hashes function that merges two JSON objects (hashes) together.
use color_eyre::owo_colors::OwoColorize;
use log::{error, trace};
use serde_json::{Map, Value};

use crate::config::Config;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct MergeResult {
  /// The merged hash
  pub new: Value,
  /// The old hash
  pub old: Value,
  pub reset: Value,
  /// The number of keys that were merged
  pub merged_count: usize,
  /// The number of keys that were pulled from the source
  pub unchanged_count: usize,
  /// The number of keys that were replaced
  pub replaced_count: usize,
  /// The number of keys that were reset
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
/// * `is_default_ns` - A boolean that indicates whether the values should be reset and flagged.
/// * `locale` - A string that represents the locale for which the merge is being performed.
/// * `config` - A reference to a Config object that contains configuration options.
///
/// # Returns
///
/// * `MergeResult` - A struct that contains the new JSON object, the old JSON object, the reset JSON object,
///   and counts of merged, pulled, old, and reset keys.
pub fn merge_hashes(
  source: Option<&Value>,
  parsed_values: &Value,
  reset_values: Option<&Value>,
  full_key_prefix: &str,
  is_principal: bool,
  locale: &str,
  config: &Config,
) -> MergeResult {
  let mut old = Map::new();
  let mut reset = Map::new();
  let mut merged_count = 0;
  let mut unchanged_count = 0;
  let mut replaced_count = 0;
  let mut reset_count = 0;
  let mut parsed_values = parsed_values.as_object().map_or_else(Map::new, |v| v.clone());

  let key_separator = &config.key_separator;
  let default_locale = config.default_locale();
  let reset_values_map = reset_values.and_then(|v| v.as_object()).map_or_else(Map::new, |v| v.clone());

  match source {
    Some(Value::Object(source_map)) => {
      for (from_source_key, from_source_value) in source_map {
        if let Some(value) = parsed_values.get(from_source_key) {
          trace!(
            "Current entry: {}: [{}`{}`] -> [{}`{}`]",
            from_source_key.italic().purple(),
            "from_code: ".bright_black(),
            value.yellow(),
            "from_store: ".bright_black(),
            from_source_value.green()
          );
        } else {
          trace!("Current entry: {}: `{}`", from_source_key.italic().purple(), from_source_value.green());
        }
        match (parsed_values.get_mut(from_source_key), from_source_value) {
          (Some(value @ Value::Object(_)), source_value @ Value::Object(_)) => {
            let full_key_prefix = &format!("{full_key_prefix}{from_source_key}{key_separator}");
            trace!("Nested: {}", from_source_key.yellow());
            let nested_result = merge_hashes(
              Some(source_value),
              value,
              reset_values_map.get(from_source_key),
              full_key_prefix,
              is_principal,
              locale,
              config,
            );
            *value = nested_result.new;
            merged_count += nested_result.merged_count;
            unchanged_count += nested_result.unchanged_count;
            replaced_count += nested_result.replaced_count;
            reset_count += nested_result.reset_count;
            match nested_result.old {
              Value::Object(old_map) if !old_map.is_empty() => {
                old.insert(from_source_key.clone(), old_map.into());
              },
              Value::Object(_) => {},
              _ => {
                error!("Old map is not an object: {:?}", nested_result.old);
                if cfg!(debug_assertions) {
                  panic!("Old map is not an object: {:?}", nested_result.old);
                }
              },
            }

            match nested_result.reset {
              Value::Object(reset_map) if !reset_map.is_empty() => {
                reset.insert(from_source_key.clone(), reset_map.into());
              },
              Value::Object(_) => {},
              _ => {
                error!("reset map is not an object: {:?}", nested_result.reset);
                if cfg!(debug_assertions) {
                  panic!("reset map is not an object: {:?}", nested_result.reset);
                }
              },
            }
          },
          (Some(target_value), source_value) if target_value == source_value => {
            trace!("{}: {} is unchanged", from_source_key.purple(), target_value.green());
            unchanged_count += 1;
          },
          (Some(Value::String(target_value)), Value::String(source_value))
            if from_source_key.contains(&config.context_separator)
              || from_source_key.contains(&config.plural_separator) =>
          {
            trace!(
              "{}: {} -> {}",
              from_source_key.purple(),
              source_value.red().italic().strikethrough(),
              target_value.green(),
            );
            *target_value = source_value.clone();
            unchanged_count += 1;
          },
          (Some(target_value), source_value) if is_principal || reset_values_map.contains_key(from_source_key) => {
            trace!(
              "{}: {} -> {}",
              from_source_key.purple(),
              target_value.red().italic().strikethrough(),
              source_value.cyan(),
            );
            old.insert(from_source_key.clone(), source_value.clone());
            replaced_count += 1;
            reset.insert(from_source_key.clone(), Value::Bool(true));
            reset_count += 1;
          },
          (Some(target_value), source_value @ Value::String(_)) if locale == default_locale => {
            trace!(
              "[Default locale] {}: {} -> {}",
              from_source_key.purple(),
              target_value.red().italic().strikethrough(),
              source_value.cyan(),
            );
            merged_count += 1;
            *target_value = source_value.clone();
          },
          (Some(target_value), source_value @ Value::Array(_)) => {
            trace!("{}: {} -> {}", from_source_key.purple(), source_value.cyan(), target_value);
            old.insert(from_source_key.clone(), source_value.clone());
            replaced_count += 1;
          },
          (Some(Value::String(target_value)), raw_source_value @ Value::String(source_value)) => {
            trace!("{}: {} -> {}", from_source_key.purple(), source_value.green(), target_value.red().strikethrough());
            old.insert(from_source_key.clone(), raw_source_value.clone());
            unchanged_count += 1;
            *target_value = source_value.clone();
          },
          (Some(target_value), source_value @ Value::String(_)) => {
            trace!("{}: {} -> {}", from_source_key.purple(), source_value.green(), target_value.red().strikethrough());
            old.insert(from_source_key.clone(), source_value.clone());
            unchanged_count += 1;
            *target_value = source_value.clone();
          },
          (Some(target_value), source_value @ Value::Object(_)) => {
            trace!("{}: {} -> {}", from_source_key.purple(), target_value.green(), source_value.red().strikethrough());
            old.insert(from_source_key.clone(), source_value.clone());
            unchanged_count += 1;
          },
          (_, source_value) => {
            trace!("Source: {source_value}");
            trace!("Pulling key: {}", from_source_key.purple());
            if config.keep_removed {
              parsed_values.insert(from_source_key.clone(), source_value.clone());
            } else {
              old.insert(from_source_key.clone(), source_value.clone());
            }
            replaced_count += 1;
          },
        }
      }
    },
    _ => {
      trace!("No source provided, returning existing hash as is.");
      trace!("Existing: {:?}", parsed_values.cyan());
    },
  }

  parsed_values.sort_keys();
  old.sort_keys();
  reset.sort_keys();
  MergeResult {
    new: Value::Object(parsed_values),
    old: Value::Object(old),
    reset: Value::Object(reset),
    merged_count,
    unchanged_count,
    replaced_count,
    reset_count,
  }
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;

  #[test_log::test]
  fn should_copies_source_keys_to_target_regardless_of_presence_when_keep_removed_is_enabled() {
    let source = json!({
          "key1": "value1",
          "key2": "value2",
          "key4": { "key41": "value41" },
    });
    let target = json!({ "key1": "", "key3": "" });

    let result =
      merge_hashes(Some(&source), &target, None, "", false, "en", &Config { keep_removed: true, ..Default::default() });

    assert_eq!(
      result.new,
      json!({
        "key1": "value1",
        "key2": "value2",
        "key3": "",
        "key4": { "key41": "value41" },
      })
    );
    assert_eq!(result.old, json!({}));
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 2);
  }

  #[test_log::test]
  fn should_copies_nested_source_keys_to_target_regardless_of_presence_when_keep_removed_is_enabled() {
    let source = json!({
          "key1": "value1",
          "key2": "value2",
          "key4": { "key41": "value41" },
    });
    let target = json!({ "key1": "", "key3": "", "key4": { "key42": "" }});

    let result =
      merge_hashes(Some(&source), &target, None, "", false, "en", &Config { keep_removed: true, ..Default::default() });

    assert_eq!(
      result.new,
      json!({
        "key1": "value1",
        "key2": "value2",
        "key3": "",
        "key4": { "key41": "value41", "key42": "" },
      })
    );
    assert_eq!(result.old, json!({}));
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 2);
  }

  #[test_log::test]
  fn should_resets_and_flags_keys_if_the_reset_and_flag_value_is_set() {
    let source = json!({ "key1": "key1", "key2": "key2" });
    let target = json!({ "key1": "changedKey1", "key2": "key2" });

    let result = merge_hashes(Some(&source), &target, None, "", true, "en", &Default::default());

    assert_eq!(result.new, json!({ "key1": "changedKey1", "key2": "key2" }));
    assert_eq!(result.old, json!({ "key1": "key1" }));
    assert_eq!(result.reset, json!({ "key1": true }));
    assert_eq!(result.reset_count, 1);
  }

  #[test_log::test]
  fn should_resets_and_flags_keys_if_the_reset_and_flag_value_is_set_with_nested() {
    let source = json!({
      "key1": {
        "key2": "key2",
      },
      "key3": {
        "key4": "key4",
      },
    });
    let target = json!({
      "key1": {
        "key2": "changedKey2",
      },
      "key3": {
        "key4": "key4",
      },
    });

    let result = merge_hashes(Some(&source), &target, None, "", true, "en", &Default::default());

    assert_eq!(
      result.new,
      json!({
        "key1": {
          "key2": "changedKey2",
        },
        "key3": {
          "key4": "key4",
        },
      })
    );
    assert_eq!(
      result.old,
      json!({
        "key1": {
          "key2": "key2",
        },
      })
    );
    assert_eq!(
      result.reset,
      json!({
        "key1": {
          "key2": true,
        },
      })
    );
    assert_eq!(result.reset_count, 1);
  }

  #[test_log::test]
  fn test_merge_hashes_no_source() {
    let existing = json!({
      "key1": "value1",
      "key2": "value2"
    });
    let config = Default::default();
    let reset_values = None;

    let result = merge_hashes(None, &existing, reset_values, "", false, "en", &config);

    assert_eq!(result.new, existing);
    assert_eq!(result.old, json!({}));
    assert_eq!(result.reset, json!({}));
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 0);
    assert_eq!(result.reset_count, 0);
  }

  #[test_log::test]
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

    let result = merge_hashes(source, &existing, reset_values, "", false, "en", &config);

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
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 1);
    assert_eq!(result.reset_count, 0);
  }

  #[test_log::test]
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

    let result = merge_hashes(source, &existing, reset_values, "", false, "en", &config);

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
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 1);
    assert_eq!(result.reset_count, 0);
  }
}

#[cfg(test)]
mod default_merge_tests {
  use pretty_assertions::assert_eq;
  use serde_json::json;

  use super::*;
  fn default_merge_hashes(source: &Value, target: &Value) -> MergeResult {
    merge_hashes(Some(source), target, None, "", false, "en", &Default::default())
  }

  #[test_log::test]
  fn should_replaces_empty_target_keys_with_source() {
    let source = json!({ "key1": "value1" });
    let target = json!({ "key1": ""});

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, source, "the new hash is not as expected");
    assert_eq!(result.old, json!({}), "the old hash is not as expected");
    assert_eq!(result.merged_count, 1, "the merge count is not as expected");
    assert_eq!(result.unchanged_count, 0, "the pull count is not as expected");
    assert_eq!(result.replaced_count, 0, "the old count is not as expected");
  }

  #[test_log::test]
  fn should_not_replace_empty_target_with_source_if_it_is_a_hash() {
    let source = json!({ "key1": { "key11": "value1" } });
    let target = json!({ "key1": "" });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key1": "" }), "the new hash is not as expected");
    assert_eq!(result.old, json!({ "key1": { "key11": "value1" } }), "the old hash is not as expected");
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 1);
    assert_eq!(result.replaced_count, 0);
  }

  #[test_log::test]
  fn should_keeps_target_keys_not_in_source() {
    let source = json!({"key1": "value1"});
    let target = json!({"key1": "", "key2": ""});

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key1": "value1", "key2": "" }), "the new hash is not as expected");
    assert_eq!(result.old, json!({}), "the old hash is not as expected");
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 0);
  }

  #[test_log::test]
  fn should_stores_into_old_the_keys_from_source_that_are_not_in_target() {
    let source = json!({"key1": "value1", "key2": "value2"});
    let target = json!({"key1": ""});

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key1": "value1" }), "the new hash is not as expected");
    assert_eq!(result.old, json!({ "key2": "value2"}), "the old hash is not as expected");
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 1);
  }

  #[test_log::test]
  fn should_restore_plural_keys_when_the_singular_one_exists() {
    let source = json!({ "key1_one": "", "key1_other": "value1" });
    let target = json!({ "key1_one": "" });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key1_one": "", }));
    assert_eq!(result.old, json!({ "key1_other": "value1" }));
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 1);
    assert_eq!(result.replaced_count, 1);
  }

  #[test_log::test]
  fn should_not_restore_plural_keys_when_the_singulare_one_does_not() {
    let source = json!({ "key1_one": "", "key1_other": "value1" });
    let target = json!({ "key2": "" });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({"key2" : ""}));
    assert_eq!(result.old, json!({ "key1_one": "", "key1_other": "value1" }));
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 2);
  }

  #[test_log::test]
  fn should_restores_context_keys_when_the_singular_one_exists() {
    let source = json!({ "key1": "", "key1_context": "value1" });
    let target = json!({ "key1": "" });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key1": "" }));
    assert_eq!(result.old, json!({ "key1_context": "value1"}));
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 1);
    assert_eq!(result.replaced_count, 1);
  }

  #[test_log::test]
  fn should_not_restores_context_keys_when_the_singular_one_does_not() {
    let source = json!({ "key1": "", "key1_context": "value1" });
    let target = json!({ "key2": "" });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(result.new, json!({ "key2": ""}));
    assert_eq!(result.old, json!({ "key1": "", "key1_context": "value1" }));
    assert_eq!(result.merged_count, 0);
    assert_eq!(result.unchanged_count, 0);
    assert_eq!(result.replaced_count, 2);
  }

  #[test_log::test]
  fn should_works_with_deep_objects() {
    let source = json!({
      "key1": "value1",
      "key2": {
        "key21": "value21",
        "key22": {
          "key221": "value221",
          "key222": "value222",
        },
        "key23": "value23",
      },
      "key4": {
        "key41": "value41",
      },
    });
    let target = json!({
      "key1": "",
      "key2": {
        "key21": "",
        "key22": {
          "key222": "",
          "key223": "",
        },
        "key24": "",
      },
      "key3": "",
      "key4": {
        "key41": "value41",
      },
    });

    let result = default_merge_hashes(&source, &target);

    assert_eq!(
      result.new,
      json!({
        "key1": "value1",
        "key2": {
          "key21": "value21",
          "key22": {
            "key222": "value222",
            "key223": "",
          },
          "key24": "",
        },
        "key3": "",
        "key4": {
          "key41": "value41",
        },
      })
    );
    assert_eq!(
      result.old,
      json!({
        "key2": {
          "key22": {
            "key221": "value221",
          },
          "key23": "value23",
        },
      })
    );
    assert_eq!(result.merged_count, 3);
    assert_eq!(result.unchanged_count, 1);
    assert_eq!(result.replaced_count, 2);
  }
}
