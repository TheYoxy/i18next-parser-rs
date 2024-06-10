#![cfg_attr(debug_assertions, allow(dead_code))]

use std::{
  collections::HashMap,
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  str::FromStr,
};

use color_eyre::eyre::Result;
use log::{debug, trace};
use serde_json::Value;

use catalog::get_catalog;
use config::Options;
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
mod tests;
mod utils;
mod visitor;

fn main() -> Result<()> {
  use clap::Parser;
  initialize_panic_handler()?;
  initialize_logging()?;
  let cli = Cli::parse();

  let config = Config::new(cli.path.to_str())?;
  debug!("Configuration: {config:?}");

  let name = "assets/file.tsx";

  let locales = vec!["en", "fr"];
  let output = "locales/$LOCALE/$NAMESPACE.json";

  let entries = parse_file(name)?;
  let options = Options::default();

  write_to_file(locales, entries, options, output);

  Ok(())
}

struct MergeAllResults {
  path: PathBuf,
  backup: PathBuf,
  merged: MergeResult,
  old_catalog: Value,
}

fn merge_all_results(
  locale: &&str,
  namespace: &String,
  catalog: &Value,
  output: &str,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  options: &Options,
) -> MergeAllResults {
  const NS_SEPARATOR: &str = ":"; // TODO: replace this const by options

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
  let merged = merge_hashes(
    value.as_ref(),
    &catalog,
    Options {
      full_key_prefix: namespace.to_string() + NS_SEPARATOR,
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
  print_counts(locale, namespace.as_str(), &unique_count, &unique_plurals_count, &merged, &old_merged, &options);
  MergeAllResults { path, backup, merged, old_catalog }
}

struct TransformEntriesResult {
  unique_count: HashMap<String, usize>,
  unique_plurals_count: HashMap<String, usize>,
  value: Value,
}
fn transform_entries(entries: &Vec<Entry>, options: &Options) -> TransformEntriesResult {
  let mut unique_count = HashMap::new();
  let mut unique_plurals_count = HashMap::new();
  let mut value = Value::Object(Default::default());

  for entry in entries {
    let namespace = entry.namespace.clone().unwrap_or("default".to_string());
    if !unique_count.contains_key(&namespace) {
      unique_count.insert(namespace.clone(), 0);
    }
    if !unique_plurals_count.contains_key(&namespace) {
      unique_plurals_count.insert(namespace.clone(), 0);
    }

    // TODO:add pluralization resolver to transform the entry

    let suffix = None;
    let result = dot_path_to_hash(entry, &value, &Options { suffix: suffix.clone(), ..options.clone() });
    trace!("Result: {:?} <- {:?}", value, result.target);
    value = result.target;

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
  }
  TransformEntriesResult { unique_count, unique_plurals_count, value }
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

  if options.line_ending == "auto" {
    // Do nothing, as Rust automatically uses the appropriate line endings
  } else if options.line_ending == "\r\n" || options.line_ending == "crlf" {
    text = text.replace('\n', "\r\n");
  } else if options.line_ending == "\r" || options.line_ending == "cr" {
    text = text.replace('\n', "\r");
  } else {
    // Defaults to LF, aka \n
    // Do nothing, as Rust already uses LF by default
  }

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
