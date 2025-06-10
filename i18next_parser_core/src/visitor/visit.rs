use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace};
use oxc_ast::ast::{CallExpression, JSXElement, JSXElementName};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;

use crate::{
  visitor::{entry::Location, I18NVisitor},
  Entry,
};

#[cfg(debug_assertions)]
pub fn print_error_location(file_path: &std::path::PathBuf, span: &oxc_span::Span) -> color_eyre::Result<()> {
  use bat::{
    line_range::{LineRange, LineRanges},
    PrettyPrinter,
  };
  let content = std::fs::read_to_string(file_path)
    .inspect_err(|e| log::error!("Unable to read file {}: {}", file_path.display(), e))?;
  let mut start_line = 1;
  let mut end_line = 1;
  let start_pos = usize::try_from(span.start)?;
  let end_pos = usize::try_from(span.end)?;
  for (i, c) in content.chars().enumerate() {
    if i == start_pos {
      start_line = end_line;
    }
    if i == end_pos {
      break;
    }

    if c == '\n' {
      end_line += 1;
    }
  }

  let bound = 2;
  let range = LineRange::from(format!("{}:{}", start_line - bound, end_line + bound).as_str()).unwrap();
  PrettyPrinter::new()
    .input_file(file_path)
    .line_ranges(LineRanges::from(vec![range]))
    .header(true)
    .grid(true)
    .line_numbers(true)
    .highlight_range(start_line, end_line)
    .print()
    .unwrap();

  Ok(())
}

impl<'a> Visit<'a> for I18NVisitor<'a> {
  fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
    const TRANSLATION_FUNCTIONS: [&str; 1] = ["t"];
    if let Some(name) = expr.callee_name() {
      self.extract_namespace(name, expr);
      if TRANSLATION_FUNCTIONS.contains(&name) {
        if let Some(key) = self.extract_t_function_key(expr) {
          trace!("Key: {key}", key = key.italic().cyan());
          let (value, i18next_options) = self.read_t_args((expr.arguments.get(1), expr.arguments.get(2)));

          let options = i18next_options.as_ref();
          let (key, namespace) = self.get_namespace(options, &key);
          let has_count = match options {
            Some(opt) => opt.get("count").is_some(),
            None => false,
          };

          let context = match options {
            Some(opt) => opt.get("context").cloned().unwrap_or(None).map(|v| vec![v]),
            None => None,
          };

          for stmt in self.program.body.iter() {
            if stmt.span() == expr.span {
              debug!("Statement: {stmt:?}");
            }
          }

          self.entries.push(Entry {
            location: Location::new(
              self.file_path.to_str().unwrap().to_string(),
              usize::try_from(expr.span.start).unwrap(),
              usize::try_from(expr.span.end).unwrap(),
            ),
            key,
            value,
            namespace,
            has_count,
            i18next_options,
            context,
          });
        }
      };
    }
    walk::walk_call_expression(self, expr);
  }

  fn visit_jsx_element(&mut self, elem: &JSXElement<'a>) {
    const COMPONENT_FUNCTIONS: [&str; 1] = ["Trans"];
    trace!("Visiting JSX Element: {:?}", elem.opening_element.name);
    match &elem.opening_element.name {
      JSXElementName::Identifier(id) if COMPONENT_FUNCTIONS.contains(&id.name.as_ref()) => {
        self.extract_jsx_entries(elem);
      },
      JSXElementName::IdentifierReference(id) if COMPONENT_FUNCTIONS.contains(&id.name.as_ref()) => {
        self.extract_jsx_entries(elem);
      },
      _ => {
        trace!("Skipping JSX Element: {:?}", elem.opening_element.name);
      },
    };
    walk::walk_jsx_element(self, elem);
  }
}
