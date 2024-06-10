#![cfg(test)]

use flatten_json_object::Flattener;
use log::LevelFilter;
use oxc_ast::Visit;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  str::FromStr,
};

use catalog::get_catalog;
use helper::{dot_path_to_hash, merge_hashes, Options};
use log::{info, trace};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;
use simple_logger::SimpleLogger;
use visitor::I18NVisitor;

use crate::{catalog, helper, visitor, IsEmpty, NS_SEPARATOR};

#[test]
fn should_parse() {
  let level = LevelFilter::Trace;

  SimpleLogger::new().with_level(level).env().init().unwrap();

  let name = "assets/file.tsx";
  let path = Path::new(&name);
  let source_text = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("Unable to find {name}"));
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

  assert_eq!(locales.len(), 2);
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
      println!("{:?}", catalog);
      assert_eq!(catalog.len(), 1, "expected 1 namespace for locale {locale}");
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
        let _old_merged =
          merge_hashes(old_value.as_ref(), &merged.new, Options { keep_removed: None, ..Default::default() }, None);
        assert_eq!(merged.old_count, 0, "there isn't any values yet");
        assert_eq!(merged.merge_count, 0, "there is 0 values to merge");
        assert_eq!(merged.pull_count, 0, "there is 8 new values");
        assert_eq!(merged.reset_count, 0, "there is 0 values to reset");
        assert!(merged.old.is_empty(), "there isn't any old values");
        assert!(merged.reset.is_empty(), "there isn't any reset values");
        assert!(!merged.new.is_empty(), "values must be parsed");
        let new = Flattener::new().set_key_separator(".").flatten(&merged.new).unwrap();
        let new = new.as_object().unwrap();
        println!("New: {:?}", new);
        for key in [
          "toast.title",
          "toast.validation.error",
          "toast.text.success",
          "toast.text.error",
          "dialog.title",
          "dialog.description",
          "button.clear",
          "button.submit",
        ] {
          assert!(new.contains_key(key), "the key {key:?} is present in the new catalog");
        }
      }
    }
  }
}
