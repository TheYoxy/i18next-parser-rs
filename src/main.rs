#![cfg_attr(debug_assertions, allow(dead_code))]

mod catalog;
mod cli;
mod helper;
mod tests;
mod visitor;
use std::{
  collections::HashMap,
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  str::FromStr,
};

use catalog::get_catalog;
use helper::{dot_path_to_hash, merge_hashes, MergeResult, Options};
use log::{info, trace, LevelFilter};
use oxc_allocator::Allocator;
use oxc_ast::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;
use simple_logger::SimpleLogger;
use visitor::I18NVisitor;

const NS_SEPARATOR: &str = ":";

fn main() -> Result<(), String> {
  // let cli = Cli::parse();
  #[cfg(debug_assertions)]
  let level = LevelFilter::Debug;
  #[cfg(not(debug_assertions))]
  let level = LevelFilter::Warn;

  SimpleLogger::new().with_level(level).env().init().unwrap();

  let name = "assets/file.tsx";
  let path = Path::new(&name);
  let source_text = std::fs::read_to_string(path).map_err(|_| format!("Unable to find {name}"))?;
  let allocator = Allocator::default();
  let source_type = SourceType::from_path(path).unwrap();
  let ret = Parser::new(&allocator, &source_text, source_type).parse();

  let locales = ["en", "fr"];
  let output = "locales/$LOCALE/$NAMESPACE.json";

  for error in ret.errors {
    let error = error.with_source_code(source_text.clone());
    eprintln!("{error:?}");
  }

  let program = ret.program;

  let mut visitor = I18NVisitor::new(&program);

  info!("Start parsing...");
  let now = std::time::Instant::now();
  visitor.visit_program(&program);
  let elapsed_time = now.elapsed();
  info!("File parsed in {}ms.", elapsed_time.as_millis());
  info!("Found {} entries", visitor.entries.len());

  let options = Options::default();

  let entries = visitor.entries;

  for locale in locales.iter() {
    let mut unique_count = HashMap::new();
    let mut unique_plurals_count = HashMap::new();
    let mut target = Value::Object(Default::default());

    for entry in &entries {
      let namespace = entry.namespace.clone().unwrap_or("default".to_string());
      if !unique_count.contains_key(&namespace) {
        unique_count.insert(namespace.clone(), 0);
      }
      if !unique_plurals_count.contains_key(&namespace) {
        unique_plurals_count.insert(namespace.clone(), 0);
      }

      let suffix = None;
      let result = dot_path_to_hash(entry, &target, &Options { suffix: suffix.clone(), ..options.clone() });
      trace!("Result: {:?} <- {:?}", target, result.target);
      target = result.target;
      if result.duplicate {
        match result.conflict {
          Some(val) if val == "key" => println!(
            "Found translation key already mapped to a map or parent of new key already mapped to a string: {}",
            entry.key
          ),
          Some(val) if val == "value" => println!("Found same keys with different values: {}", entry.key),
          _ => (),
        }
      } else {
        *unique_count.get_mut(&namespace).unwrap() += 1;
        if suffix.is_some() {
          *unique_plurals_count.get_mut(&namespace).unwrap() += 1;
        }
      }
    }

    if let Value::Object(catalog) = target {
      for (namespace, catalog) in catalog {
        let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", &namespace.clone());
        trace!("Path for output {output:?}: {path:?}");
        // TODO:add pluralization resolver to transform the entry
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
        // todo: add failOnUpdate / failOnWarnings
        push_file(&path, &merged.new, &options).unwrap_or_else(|_| panic!("Unable to write file {path:?}"));
        if options.create_old_catalogs && !old_catalog.is_empty() {
          push_file(&backup, &old_catalog, &options).unwrap_or_else(|_| panic!("Unable to write file {backup:?}"));
        }
      }
    }
  }

  // aa(&module);
  // println!("Module {:?}", module);

  Ok(())
}

trait IsEmpty {
  fn is_empty(&self) -> bool;
}

impl IsEmpty for Value {
  fn is_empty(&self) -> bool {
    match self {
      Value::Null => true,
      Value::Bool(_) => false,
      Value::Number(_) => false,
      Value::String(_) => false,
      Value::Array(arr) => arr.is_empty(),
      Value::Object(obj) => obj.is_empty(),
    }
  }
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
