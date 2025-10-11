use std::collections::HashMap;

use color_eyre::owo_colors::OwoColorize;
use i18next_parser_core::MergeResult;
use serde_json::{Map, Value};

const PLURAL_SUFFIXES: &[&str] = &["zero", "one", "two", "few", "many", "other"];

fn is_plural(key: &str) -> bool {
  PLURAL_SUFFIXES.iter().any(|suffix| key.ends_with(suffix))
}

fn count_entries(value: &Map<String, Value>) -> (usize, usize) {
  value
    .iter()
    .fold((0, 0), |(mut prev_single, mut prev_multiple), (key, curr_value)| {
      match curr_value {
        Value::String(_) if is_plural(key) => {
          prev_multiple += 1;
        }
        Value::String(_) => {
          prev_single += 1;
        }
        Value::Object(obj) => {
          let (single, multiple) = count_entries(obj);
          prev_single += single;
          prev_multiple += multiple;
        }
        _ => {}
      }
      (prev_single, prev_multiple)
    })
}

pub type CountResults<'a> = HashMap<&'a String, HashMap<&'a String, &'a MergeResult>>;
pub trait PrintCounts {
  fn print_counts(&self);
}

impl PrintCounts for CountResults<'_> {
  fn print_counts(&self) {
    self.iter().for_each(|(namespace, map)| {
      tracing::info!(target: "count", "{}", namespace.blue().bold());
      map.iter().for_each(|(locale, merged)| {
        print_counts_inner(locale, merged);
      });
    });
  }
}

pub fn print_counts_inner(locale: &str, merged: &MergeResult) {
  let (unique_count, unique_plurals_count) = count_entries(merged.new.as_object().expect("should be an object"));

  let add_count = merged.merged_count;
  let modified_count = merged.replaced_count;
  let deleted_count = merged.reset_count;

  let keys = format!("Keys: {}", unique_count.underline());
  let plurals = if unique_plurals_count == 0 {
    "".into()
  } else {
    format!("({unique_plurals_count} are plurals)")
  };

  let diff = format!(
    "{}{} {}{} {}{}",
    "+".green(),
    add_count.green(),
    "~".yellow(),
    modified_count.yellow(),
    "-".red(),
    deleted_count.red()
  );

  tracing::info!(target: "count", "\t[{}] {} {} {}", locale.cyan().italic(), keys, diff, plurals.bright_black());
}

#[cfg(test)]
mod test_count_entries {
  use super::*;

  #[test]
  fn should_count_entries_from_nested_object() {
    let value = serde_json::json!({
        "key1": "value1",
        "key2": {
            "subkey1": "value2",
            "subkey2": {
                "subsubkey1": "value3"
            }
        },
    });

    let (count, _) = count_entries(&value.as_object().expect("should be an object"));
    assert_eq!(count, 3);
  }

  #[test]
  fn should_count_entries_from_simple_object() {
    let value = serde_json::json!({
        "key1": "value1",
    });

    let (count, _) = count_entries(&value.as_object().expect("should be an object"));
    assert_eq!(count, 1);
  }

  #[test]
  fn should_count_entries_with_plurals() {
    let value = serde_json::json!({
        "key1": "value1",
        "key2_many": "value2"
    });

    let (count, count_multiple) = count_entries(&value.as_object().expect("should be an object"));
    assert_eq!(count, 1);
    assert_eq!(count_multiple, 1);
  }

  #[test]
  fn should_count_entries_from_complex_nested_object() {
    let value = serde_json::json!({
        "key1": {
            "key2": {
                "key3": {
                    "key4": {
                        "key5": "value5"
                    }
                }
            }
        },
    });

    let (count, _) = count_entries(&value.as_object().expect("should be an object"));
    assert_eq!(count, 1);
  }
}
