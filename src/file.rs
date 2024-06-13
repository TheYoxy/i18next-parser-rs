use crate::config::Options;
use crate::visitor::{Entry, I18NVisitor};
use crate::{helper::log_execution_time, is_empty::IsEmpty};
use crate::{merge_all_results, push_file, transform_entries, MergeAllResults, TransformEntriesResult};
use log::info;
use oxc_ast::Visit;
use serde_json::Value;
use std::path::Path;

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
  log_execution_time(format!("Parsing file {file_name}").as_str(), || {
    visitor.visit_program(visitor.program);
  });
  info!("Found {} entries", visitor.entries.len());

  Ok(visitor.entries)
}

pub fn write_to_file(entries: Vec<Entry>, options: Options) {
  log_execution_time("Writing files", || {
    let locales = &options.locales;
    for locale in locales.iter() {
      let TransformEntriesResult { unique_count, unique_plurals_count, value } =
        transform_entries(&entries, locale, &options);

      if let Value::Object(catalog) = value {
        for (namespace, catalog) in catalog {
          let MergeAllResults { path, backup, merged, old_catalog } =
            merge_all_results(locale, &namespace, &catalog, &unique_count, &unique_plurals_count, &options);

          push_file(&path, &merged.new, &options).unwrap_or_else(|_| panic!("Unable to write file {path:?}"));
          if options.create_old_catalogs && !old_catalog.is_empty() {
            push_file(&backup, &old_catalog, &options).unwrap_or_else(|_| panic!("Unable to write file {backup:?}"));
          }
        }
      }
    }
  });
}
