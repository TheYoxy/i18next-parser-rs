use crate::visitor::{Entry, I18NVisitor};
use crate::{
  catalog::get_catalog,
  config::Options,
  helper::{merge_hashes, MergeResult},
  print_counts, transfer_values,
};
use crate::{helper::log_execution_time, is_empty::IsEmpty};
use crate::{push_file, transform_entries, TransformEntriesResult};
use log::{info, trace};
use oxc_ast::Visit;
use serde_json::Value;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  str::FromStr,
};

pub fn parse_file<P>(path: P) -> color_eyre::Result<Vec<Entry>>
where
  P: AsRef<Path>,
{
  use oxc_allocator::Allocator;
  use oxc_parser::Parser;
  use oxc_span::SourceType;
  use std::fs::read_to_string;

  let source_text = read_to_string(&path)?;
  let allocator = &Allocator::default();
  let source_type = SourceType::from_path(&path).unwrap();
  let parser = Parser::new(allocator, source_text.as_str(), source_type);
  let parsed = parser.parse();
  let mut visitor = I18NVisitor::new(&parsed.program);
  visitor.visit_program(&parsed.program);

  info!("Start parsing...");
  let file_name = path.as_ref().file_name().and_then(|s| s.to_str()).unwrap();
  log_execution_time(format!("Parsing file {file_name}"), || {
    visitor.visit_program(visitor.program);
  });
  info!("Found {} entries", visitor.entries.len());

  Ok(visitor.entries)
}

/// Parse a directory and return a list of entries.
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub fn parse_directory(path: &std::path::PathBuf, config: &crate::config::Config) -> color_eyre::Result<Vec<Entry>> {
  let inputs = &config.input;
  let mut builder = globset::GlobSetBuilder::new();
  for input in inputs {
    let join = path.join(input);
    let glob = join.to_str().unwrap();
    builder.add(globset::Glob::new(glob)?);
  }

  let globset = builder.build()?;

  let file_name = path.file_name().and_then(|s| s.to_str()).unwrap();
  let entries = log_execution_time(format!("Directory {file_name}"), || {
    ignore::WalkBuilder::new(path)
      .standard_filters(true)
      .build()
      .filter_map(Result::ok)
      .filter(|f| globset.is_match(f.path()))
      .filter_map(|entry| {
        let entry_path = entry.path();
        crate::printinfo!("Reading file: {:?}", entry_path);
        parse_file(entry_path).ok()
      })
      .flatten()
      .collect::<Vec<_>>()
  });

  Ok(entries)
}

/// Write all entries to the specific file based on its namespace
///
/// # Panics
///
/// Panics if .
pub fn write_to_file(entries: Vec<Entry>, options: Options) -> color_eyre::Result<()> {
  log_execution_time("Writing files", || {
    let locales = &options.locales;
    for locale in locales.iter() {
      let TransformEntriesResult { unique_count, unique_plurals_count, value } =
        transform_entries(&entries, locale, &options);

      if let Value::Object(catalog) = value {
        for (namespace, catalog) in catalog {
          let MergeAllResults { path, backup, merged, old_catalog } =
            merge_all_results(locale, &namespace, &catalog, &unique_count, &unique_plurals_count, &options);

          push_file(&path, &merged.new, &options)?;
          if options.create_old_catalogs && !old_catalog.is_empty() {
            push_file(&backup, &old_catalog, &options)?;
          }
        }
      }
    }
    Ok(())
  })
}

pub struct MergeAllResults {
  pub path: PathBuf,
  pub backup: PathBuf,
  pub merged: MergeResult,
  pub old_catalog: Value,
}

pub fn merge_all_results(
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
  let old_value = old_value.as_ref();

  trace!("Value: {value:?} -> {old_value:?}");
  let ns_separator = options.key_separator.as_deref().unwrap_or("");
  let merged: MergeResult = merge_hashes(
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
    old_value,
  );
  let old_merged = merge_hashes(old_value, &merged.new, Options { keep_removed: None, ..Default::default() }, None);
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if options.verbose {
    print_counts(locale, namespace.as_str(), unique_count, unique_plurals_count, &merged, &old_merged, options);
  }

  MergeAllResults { path, backup, merged, old_catalog }
}
