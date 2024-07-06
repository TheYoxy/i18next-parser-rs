use std::path::Path;

use log::trace;
use oxc_ast::Visit;
use tracing::instrument;

use crate::{
  log_time,
  visitor::{Entry, I18NVisitor},
};

#[instrument(skip(path), err)]
pub(crate) fn parse_file<P>(path: P) -> color_eyre::Result<Vec<Entry>>
where
  P: AsRef<Path>,
{
  use std::fs::read_to_string;

  use oxc_allocator::Allocator;
  use oxc_parser::Parser;
  use oxc_span::SourceType;

  let file_name = path.as_ref().file_name().and_then(|s| s.to_str()).unwrap();
  let source_text = log_time!(format!("Reading file {file_name}"), { read_to_string(&path) })?;

  let allocator = &Allocator::default();
  let source_type = SourceType::from_path(&path).unwrap();
  let parser = Parser::new(allocator, source_text.as_str(), source_type);
  let parsed = parser.parse();
  let mut visitor = I18NVisitor::new(&parsed.program);

  trace!("Start parsing...");
  log_time!(format!("Parsing file {file_name}"), {
    visitor.visit_program(&parsed.program);
  });
  trace!("Found {} entries", visitor.entries.len());

  Ok(visitor.entries)
}
