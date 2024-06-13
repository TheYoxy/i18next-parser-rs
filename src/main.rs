#![cfg_attr(debug_assertions, allow(dead_code))]
use clap::Parser;
use globset::Glob;
use ignore::WalkBuilder;

use std::{
  collections::HashMap,
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  str::FromStr,
};

use color_eyre::eyre::Result;
use globset::GlobSetBuilder;
use log::{debug, info, trace};
use serde_json::Value;

use catalog::get_catalog;
use config::{LineEnding, Options};
use helper::{dot_path_to_hash, merge_hashes, MergeResult};

use crate::file::{parse_file, write_to_file};
use crate::visitor::Entry;
use crate::{
  cli::Cli,
  config::Config,
  utils::{initialize_logging, initialize_panic_handler},
};

mod catalog;
mod cli;
mod config;
mod file;
mod helper;
mod is_empty;
mod macros;
mod plural;
mod tests;
mod utils;
mod visitor;

fn main() -> Result<()> {
  initialize_panic_handler()?;
  initialize_logging()?;
  let cli = Cli::parse();
  let path = &cli.path;
  println!("Looking for translations in path {path:?}");

  info!("Working directory: {:?}", path);
  let config = &Config::new(path)?;
  debug!("Configuration: {config:?}");

  debug!("Actual configuration: {config:?}");
  let entries = parse_directory(path, config)?;
  write_to_file(entries, config.into());

  Ok(())
}

fn parse_directory(path: &PathBuf, config: &Config) -> Result<Vec<Entry>> {
  let inputs = &config.input;
  let mut builder = GlobSetBuilder::new();
  for input in inputs {
    let join = path.join(input);
    let glob = join.to_str().unwrap();
    builder.add(Glob::new(glob)?);
  }

  let globset = builder.build()?;

  let now = std::time::Instant::now();
  let elapsed_time = now.elapsed();
  let entries = WalkBuilder::new(path)
    .standard_filters(true)
    .build()
    .filter_map(Result::ok)
    .filter(|f| globset.is_match(f.path()))
    .filter_map(|entry| {
      let entry_path = entry.path();
      printinfo!("Reading file: {:?}", entry_path);
      parse_file(entry_path).ok()
    })
    .flatten()
    .collect::<Vec<_>>();
  let file_name = path.file_name().and_then(|s| s.to_str()).unwrap();
  info!("Directory {} parsed in {}ms.", file_name, elapsed_time.as_millis());

  Ok(entries)
}

struct MergeAllResults {
  path: PathBuf,
  backup: PathBuf,
  merged: MergeResult,
  old_catalog: Value,
}

fn merge_all_results(
  locale: &str,
  namespace: &String,
  catalog: &Value,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  options: &Options,
) -> MergeAllResults {
  let output = &options.output;
  let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", &namespace.clone());
  trace!("Path for output {output:?}: {path:?}");
  let path = PathBuf::from_str(&path).unwrap_or_else(|_| panic!("Unable to find path {path:?}"));
  // get backup file name
  let filename = {
    let filename = path.file_stem().and_then(|o| o.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|o| o.to_str()).unwrap_or_default();
    format!("{}_old.{}", filename, extension)
  };
  let backup = path.with_file_name(filename);
  trace!("File path: {path:?}");
  trace!("Backup path: {backup:?}");

  let value = get_catalog(&path);
  let old_value = get_catalog(&backup);
  trace!("Value: {:?}", value);
  trace!("Old value: {:?}", old_value);
  let ns_separator = options.key_separator.as_deref().unwrap_or("");
  let merged = merge_hashes(
    value.as_ref(),
    catalog,
    Options {
      full_key_prefix: namespace.to_string() + ns_separator,
      reset_and_flag: false,
      keep_removed: None,
      key_separator: None,
      plural_separator: None,
      ..Default::default()
    },
    old_value.as_ref(),
  );
  let old_merged =
    merge_hashes(old_value.as_ref(), &merged.new, Options { keep_removed: None, ..Default::default() }, None);
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if options.verbose {
    print_counts(locale, namespace.as_str(), unique_count, unique_plurals_count, &merged, &old_merged, options);
  }
  MergeAllResults { path, backup, merged, old_catalog }
}

struct TransformEntriesResult {
  unique_count: HashMap<String, usize>,
  unique_plurals_count: HashMap<String, usize>,
  value: Value,
}

fn transform_entries(entries: &Vec<Entry>, locale: &str, options: &Options) -> TransformEntriesResult {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();
  let mut value = Value::Object(Default::default());

  for entry in entries {
    value = if options.plural_separator.is_some() && entry.count.is_some() {
      let resolver = plural::PluralResolver::default();

      for suffix in resolver.get_suffixes(locale) {
        value = transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, options, Some(suffix))
      }
      value
    } else {
      transform_entry(entry, &mut unique_count, &mut unique_plurals_count, &value, options, None)
    };
  }
  TransformEntriesResult { unique_count, unique_plurals_count, value }
}

fn transform_entry(
  entry: &Entry,
  unique_count: &mut HashMap<String, usize>,
  unique_plurals_count: &mut HashMap<String, usize>,
  value: &Value,
  options: &Options,
  suffix: Option<String>,
) -> Value {
  let namespace = entry.namespace.clone().unwrap_or("default".to_string());
  if !unique_count.contains_key(&namespace) {
    unique_count.insert(namespace.clone(), 0);
  }
  if !unique_plurals_count.contains_key(&namespace) {
    unique_plurals_count.insert(namespace.clone(), 0);
  }

  let options = if let Some(suffix) = &suffix {
    Options { suffix: Some(suffix.clone()), ..options.clone() }
  } else {
    options.clone()
  };
  let result = dot_path_to_hash(entry, value, &options);
  trace!("Result: {:?} <- {:?}", value, result.target);

  if result.duplicate {
    match result.conflict {
      Some(val) if val == "key" => printwarn!(
        "Found translation key already mapped to a map or parent of new key already mapped to a string: {}",
        entry.key
      ),
      Some(val) if val == "value" => printwarn!("Found same keys with different values: {}", entry.key),
      _ => (),
    }
  } else {
    *unique_count.get_mut(&namespace).unwrap() += 1;
    if suffix.is_some() {
      *unique_plurals_count.get_mut(&namespace).unwrap() += 1;
    }
  }

  result.target
}

fn transfer_values(source: &Value, target: &Value) -> Value {
  if let (Value::Object(source_map), Value::Object(target_map)) = (source, target) {
    let mut new_target_map = target_map.clone();
    for (key, source_value) in source_map {
      if !new_target_map.contains_key(key) {
        new_target_map.insert(key.clone(), source_value.clone());
      } else {
        let target_value = new_target_map.get_mut(key).unwrap();
        let transferred_value = transfer_values(source_value, target_value);
        *target_value = transferred_value;
      }
    }
    Value::Object(new_target_map)
  } else {
    target.clone()
  }
}

fn push_file(path: &PathBuf, contents: &Value, options: &Options) -> std::io::Result<()> {
  use std::fs::create_dir_all;
  let mut text: String;
  if path.ends_with("yml") {
    text = serde_yaml::to_string(contents).unwrap();
  } else {
    text = serde_json::to_string_pretty(contents).unwrap();
    text = text.replace("\r\n", "\n").replace('\r', "\n");
  }

  text = match options.line_ending {
    LineEnding::Crlf => text.replace('\n', "\r\n"),
    LineEnding::Cr => text.replace('\n', "\r"),
    _ => {
      // Do nothing, as Rust automatically uses the appropriate line endings
      text
    },
  };

  if let Some(parent) = path.parent() {
    if !parent.exists() {
      trace!("creating parent directory: {:?}", parent);
      create_dir_all(parent)?;
    }
  }
  let mut file = File::create(Path::new(path))?;
  file.write_all(text.as_bytes())?;

  Ok(())
}

fn print_counts(
  locale: &str,
  namespace: &str,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  merged: &MergeResult,
  old_merged: &MergeResult,
  options: &Options,
) {
  let merge_count = merged.merge_count;
  let restore_count = old_merged.merge_count;
  let old_count = merged.old_count;
  let reset_count = merged.reset_count;
  println!("[{}] {}", locale, namespace);
  let unique_count = unique_count.get(namespace).unwrap_or(&0);
  let unique_plurals_count = unique_plurals_count.get(namespace).unwrap_or(&0);
  println!("Unique keys: {} ({} are plurals)", unique_count, unique_plurals_count);
  let add_count = unique_count.saturating_sub(merge_count);
  println!("Added keys: {}", add_count);
  println!("Restored keys: {}", restore_count);
  if options.keep_removed.is_some() {
    println!("Unreferenced keys: {}", old_count);
  } else {
    println!("Removed keys: {}", old_count);
  }
  if options.reset_default_value_locale.is_some() {
    println!("Reset keys: {}", reset_count);
  }
  println!();
}

#[cfg(test)]
mod transform_entries {
  use super::*;

  #[test]
  fn test_transform_entries() {
    let entries = vec![
      Entry {
        namespace: Some("default".to_string()),
        key: "key1".to_string(),
        count: None,
        default_value: Some("value1".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("default".to_string()),
        key: "key2".to_string(),
        count: Some(3),
        default_value: Some("value2".to_string()),
        i18next_options: None,
      },
      Entry {
        namespace: Some("custom".to_string()),
        key: "key3".to_string(),
        count: None,
        default_value: Some("value3".to_string()),
        i18next_options: None,
      },
    ];
    let locale = "en";
    let options = Options { plural_separator: Some("_".to_string()), ..Options::default() };

    let result = transform_entries(&entries, locale, &options);

    assert_eq!(result.unique_count.get("default"), Some(&3));
    assert_eq!(result.unique_count.get("custom"), Some(&1));
    assert_eq!(result.unique_plurals_count.get("default"), Some(&2));
    assert_eq!(result.unique_plurals_count.get("custom"), Some(&0));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key1")), Some(&Value::String("value1".to_string())));
    assert_eq!(result.value.get("default").and_then(|v| v.get("key2")), Some(&Value::String("value2".to_string())));
    assert_eq!(result.value.get("custom").and_then(|v| v.get("key3")), Some(&Value::String("value3".to_string())));
  }
}

#[cfg(test)]
mod transform_entry {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_transform_entry() {
    let entry = Entry {
      namespace: Some("default".to_string()),
      key: "key1".to_string(),
      default_value: Some("value1".to_string()),
      count: None,
      i18next_options: None,
    };
    let mut unique_count = HashMap::new();
    let mut unique_plurals_count = HashMap::new();
    let value = Value::Object(Default::default());
    let options = Options::default();

    let result = transform_entry(&entry, &mut unique_count, &mut unique_plurals_count, &value, &options, None);

    assert_eq!(
      result,
      json!({
        "default": {
          "key1": "value1"
        }
      })
    );
    assert_eq!(unique_count.get("default"), Some(&1));
    assert_eq!(unique_plurals_count.get("default"), Some(&0));
  }
}
