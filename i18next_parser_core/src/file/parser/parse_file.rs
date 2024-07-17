use std::{fs::read_to_string, path::Path};

use color_eyre::owo_colors::OwoColorize;
use log::trace;
use oxc_allocator::Allocator;
use oxc_ast::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use tracing::instrument;

use crate::{log_time, visitor::I18NVisitor, Entry};

#[instrument(skip(path), err)]
pub fn parse_file<P>(path: P) -> color_eyre::Result<Vec<Entry>>
where
  P: AsRef<Path>,
{
  let path = path.as_ref();
  let file_name = path.file_name().and_then(|s| s.to_str()).unwrap();
  let source_text = log_time!(format!("Reading file {}", file_name.yellow().italic()), { read_to_string(path) })?;

  let allocator = &Allocator::default();
  let source_type = SourceType::from_path(path).unwrap();
  let parser = Parser::new(allocator, source_text.as_str(), source_type);
  let parsed = parser.parse();
  let mut visitor = I18NVisitor::new(&parsed.program, path);

  trace!("Start parsing file {}...", file_name.yellow().italic());
  log_time!(format!("Parsing file {}", file_name.yellow()), {
    visitor.visit_program(&parsed.program);
  });
  trace!("Found {} entries", visitor.entries.len().cyan());

  Ok(visitor.entries)
}
