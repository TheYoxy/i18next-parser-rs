use std::{
  collections::HashMap,
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  str::FromStr,
};

use color_eyre::{eyre::eyre, Report};
use log::{info, trace};
use oxc_ast::Visit;
use serde_json::Value;

use crate::transform::transfer_values::transfer_values;
use crate::transform::transform_entries::{transform_entries, TransformEntriesResult};
use crate::{
  catalog::get_catalog,
  config::Config,
  config::LineEnding,
  helper::log_execution_time::log_execution_time,
  helper::merge_hashes::{merge_hashes, MergeResult},
  is_empty::IsEmpty,
  print_counts, printinfo,
  visitor::{Entry, I18NVisitor},
};

///
///
/// # Arguments
///
/// * `path`:
///
/// returns: Result<Vec<Entry, Global>, Report>
///
/// # Examples
///
/// ```
///
/// ```
fn parse_file<P>(path: P) -> color_eyre::Result<Vec<Entry>>
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
pub(crate) fn parse_directory(path: &PathBuf, config: &Config) -> color_eyre::Result<Vec<Entry>> {
  let inputs = &config.input;
  let mut builder = globset::GlobSetBuilder::new();
  for input in inputs {
    let join = path.join(input);
    let glob = join.to_str().unwrap();
    builder.add(globset::Glob::new(glob)?);
  }

  let glob = builder.build()?;

  let directory_name = path.as_path().file_name().and_then(|s| s.to_str()).unwrap();
  let entries = log_execution_time(format!("Reading directory {directory_name}"), || {
    let filter = ignore::WalkBuilder::new(path)
      .standard_filters(true)
      .build()
      .filter_map(Result::ok)
      .filter(|f| glob.is_match(f.path()))
      .collect::<Vec<_>>();
    printinfo!("Reading {} files", filter.len());

    if filter.is_empty() {
      None
    } else {
      let entries = filter
        .iter()
        .filter_map(|entry| {
          let entry_path = entry.path();
          if config.verbose {
            crate::printread!("{}", entry_path.display());
          }
          parse_file(entry_path).ok()
        })
        .flatten()
        .collect::<Vec<_>>();
      Some(entries)
    }
  });

  entries.ok_or(eyre!("No entries found in the directory"))
}

/// Write all entries to the specific file based on its namespace
pub(crate) fn write_to_file(entries: Vec<Entry>, config: &Config) -> color_eyre::Result<()> {
  log_execution_time("Writing files", || {
    for result in prepare_to_write(entries, config)? {
      let MergeAllResults { locale: _locale, path, backup, merged, old_catalog } = result;
      write_files(&path, &backup, &merged, &old_catalog, config)?;
    }

    Ok(())
  })
}

pub(crate) fn prepare_to_write(entries: Vec<Entry>, config: &Config) -> color_eyre::Result<Vec<MergeAllResults>> {
  let mut vec: Vec<MergeAllResults> = vec![];
  log_execution_time("Preparing entries to write", || {
    let locales = &config.locales;
    for locale in locales.iter() {
      let TransformEntriesResult { unique_count, unique_plurals_count, value } =
        transform_entries(&entries, locale, config);

      if let Value::Object(catalog) = value {
        for (namespace, catalog) in catalog {
          let result = merge_all_results(locale, &namespace, &catalog, &unique_count, &unique_plurals_count, config);
          vec.push(result);
        }
      }
    }
    Ok(vec)
  })
}

fn write_files(
  path: &PathBuf,
  backup: &PathBuf,
  merged: &MergeResult,
  old_catalog: &Value,
  config: &Config,
) -> Result<(), Report> {
  let new_catalog = &merged.new;
  push_file(path, new_catalog, config)?;
  if config.create_old_catalogs && !old_catalog.is_empty() {
    push_file(backup, old_catalog, config)?;
  }
  Ok(())
}

pub(crate) struct MergeAllResults {
  pub(crate) locale: String,
  pub(crate) path: PathBuf,
  pub(crate) backup: PathBuf,
  pub(crate) merged: MergeResult,
  pub(crate) old_catalog: Value,
}

pub(crate) fn merge_all_results(
  locale: &str,
  namespace: &str,
  catalog: &Value,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  config: &Config,
) -> MergeAllResults {
  let output = &config.get_output();
  let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", namespace);
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

  let ns_separator = config.key_separator.as_deref().unwrap_or("");
  let full_key_prefix = namespace.to_string() + ns_separator;
  let merged: MergeResult = merge_hashes(value.as_ref(), catalog, old_value, &full_key_prefix, false, config);
  let old_merged = merge_hashes(
    old_value,
    &merged.new,
    None,
    &full_key_prefix,
    false,
    &Config { keep_removed: false, ..Default::default() },
  );
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if config.verbose {
    print_counts(locale, namespace, unique_count, unique_plurals_count, &merged, &old_merged, config);
  }

  MergeAllResults { locale: locale.to_string(), path, backup, merged, old_catalog }
}

fn handle_line_ending(text: &str, line_ending: &LineEnding) -> String {
  match line_ending {
    LineEnding::Crlf => text.replace('\n', "\r\n"),
    LineEnding::Cr => text.replace('\n', "\r"),
    _ => {
      // Do nothing, as Rust automatically uses the appropriate line endings
      text.to_string()
    },
  }
}

fn push_file(path: &PathBuf, contents: &Value, config: &Config) -> std::io::Result<()> {
  let text = {
    let text = if path.ends_with("yml") {
      serde_yaml::to_string(contents).unwrap()
    } else {
      serde_json::to_string_pretty(contents).map(|t| t.replace("\r\n", "\n").replace('\r', "\n")).unwrap()
    };

    handle_line_ending(&text, &config.line_ending)
  };

  if let Some(parent) = path.parent() {
    if !parent.exists() {
      trace!("creating parent directory: {:?}", parent);
      std::fs::create_dir_all(parent)?;
    }
  }
  let mut file = File::create(Path::new(path))?;
  file.write_all(text.as_bytes())?;

  Ok(())
}
