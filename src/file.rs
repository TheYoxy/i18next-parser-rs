use crate::config::Options;
use crate::is_empty::IsEmpty;
use crate::visitor::{Entry, I18NVisitor};
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
  let now = std::time::Instant::now();
  visitor.visit_program(visitor.program);
  let elapsed_time = now.elapsed();
  let file_name = path.as_ref().file_name().and_then(|s| s.to_str()).unwrap();
  info!("File {} parsed in {}ms.", file_name, elapsed_time.as_millis());
  info!("Found {} entries", visitor.entries.len());

  Ok(visitor.entries)
}

pub fn write_to_file(locales: Vec<&str>, entries: Vec<Entry>, options: Options, output: &str) {
  for locale in locales.iter() {
    let TransformEntriesResult { unique_count, unique_plurals_count, value } = transform_entries(&entries, &options);

    if let Value::Object(catalog) = value {
      for (namespace, catalog) in catalog {
        let MergeAllResults { path, backup, merged, old_catalog } =
          merge_all_results(locale, &namespace, &catalog, output, &unique_count, &unique_plurals_count, &options);
        // todo: add failOnUpdate / failOnWarnings
        push_file(&path, &merged.new, &options).unwrap_or_else(|_| panic!("Unable to write file {path:?}"));
        if options.create_old_catalogs && !old_catalog.is_empty() {
          push_file(&backup, &old_catalog, &options).unwrap_or_else(|_| panic!("Unable to write file {backup:?}"));
        }
      }
    }
  }
}
