use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::{
  ast::{Argument, CallExpression, JSXElement, JSXElementName},
  visit::walk,
  Visit,
};
use oxc_span::GetSpan;

use crate::{
  visitor::{entry::Location, I18NVisitor},
  Entry,
};

#[cfg(debug_assertions)]
pub fn print_error_location(file_path: &std::path::PathBuf, span: &oxc_span::Span) {
  use bat::{
    line_range::{LineRange, LineRanges},
    PrettyPrinter,
  };
  let content = std::fs::read_to_string(file_path).unwrap();
  let mut start_line = 1;
  let mut end_line = 1;
  let start_pos = usize::try_from(span.start).unwrap();
  let end_pos = usize::try_from(span.end).unwrap();
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
}

impl<'a> Visit<'a> for I18NVisitor<'a> {
  fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
    if let Some(name) = expr.callee_name() {
      self.extract_namespace(name, expr);
      if name == "t" {
        let key = match expr.arguments.first() {
          Some(Argument::StringLiteral(str)) => {
            trace!("t Arg: {:?}", str.bright_black().italic());
            Some(str.value.to_string().clone())
          },
          Some(Argument::TemplateLiteral(template)) => {
            trace!("t Arg: {:?}", template.bright_black().italic());
            trace!("t quasis: {:?}", template.quasis);
            trace!("t expressions: {:?}", template.expressions);
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &template.span);
              todo!("Handle template literal")
            }
            #[cfg(not(debug_assertions))]
            {
              warn!("Template literal are not supported for now");
              None
            }
          },
          Some(Argument::BinaryExpression(bin)) => {
            trace!("t Arg: {:?}", bin.bright_black().italic());
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &bin.span);
              todo!("Handle binary expression")
            }
            #[cfg(not(debug_assertions))]
            {
              warn!("Binary expression are not supported for now");
              None
            }
          },
          Some(Argument::CallExpression(expression)) => {
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &expression.span());
            }
            trace!("Skipping CallExpression as it is unsupported");
            None
          },
          Some(Argument::StaticMemberExpression(expression)) => {
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &expression.span());
            }
            trace!("Skipping StaticMemberExpression as it is unsupported");
            None
          },
          Some(Argument::Identifier(identifier)) => {
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &identifier.span());
            }
            trace!("Skipping Identifier as it is unsupported");
            None
          },
          Some(Argument::TSAsExpression(expression)) => {
            #[cfg(debug_assertions)]
            {
              print_error_location(&self.file_path, &expression.span());
            }
            trace!("Skipping TSAsExpression as it is unsupported");
            None
          },
          Some(arg) => {
            #[cfg(debug_assertions)]
            {
              log::warn!("Unknown argument type found in [{}]: {arg:?}", self.file_path.display().yellow());
              print_error_location(&self.file_path, &arg.span());

              todo!("Handle argument {arg:?} in {}", self.file_path.display().yellow())
            }
            #[cfg(not(debug_assertions))]
            {
              warn!("Unknown argument type {arg:?}");
              None
            }
          },
          None => {
            warn!("No key provided, skipping entry");
            None
          },
        };

        if let Some(key) = key {
          trace!("Key: {key}", key = key.italic().cyan());
          let (value, i18next_options) = self.read_t_args((expr.arguments.get(1), expr.arguments.get(2)));

          let options = i18next_options.as_ref();
          let (key, namespace) = self.get_namespace(options, &key);
          let has_count = match options {
            Some(opt) => opt.get("count").is_some(),
            None => false,
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
          });
        }
      };
    }
    walk::walk_call_expression(self, expr);
  }

  fn visit_jsx_element(&mut self, elem: &JSXElement<'a>) {
    let component_functions = ["Trans"];
    let name = if let JSXElementName::Identifier(id) = &elem.opening_element.name { Some(&id.name) } else { None };
    #[allow(unused_variables)]
    if let Some(name) = name {
      if component_functions.contains(&name.as_str()) {
        let key = self.get_prop_value(elem, "i18nKey");
        let ns = self.get_prop_value(elem, "ns");
        let default_value = self.get_prop_value(elem, "defaults");
        let count = self.has_prop(elem, "count");
        let options = self.get_prop_value(elem, "i18n");

        trace!("Childrens: {:?}", elem.children);
        let node_as_string = {
          let content = Self::parse_children(&elem.children);
          self.elem_to_string(&content)
        };
        trace!("Element as string: {node_as_string:?}");
        let default_value = default_value.unwrap_or(node_as_string);

        if let Some(key) = key {
          self.entries.push(Entry {
            location: Location::new(
              self.file_path.to_str().unwrap().to_string(),
              usize::try_from(elem.span.start).unwrap(),
              usize::try_from(elem.span.end).unwrap(),
            ),
            key,
            value: if default_value.is_empty() { None } else { Some(default_value) },
            namespace: ns,
            has_count: count,
            i18next_options: options.and_then(|v| serde_json::from_str(&v).ok()),
          });
        }
      }
    }

    walk::walk_jsx_element(self, elem);
  }
}
