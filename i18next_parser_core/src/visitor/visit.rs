use color_eyre::owo_colors::OwoColorize;
use log::trace;
use oxc_ast::ast::{CallExpression, JSXElement, JSXElementName};
use oxc_ast_visit::{Visit, walk};

use crate::{
  Entry,
  helper::html_entities_replacer::decode_html_entities,
  visitor::{
    I18NVisitor,
    entry::Location,
    traits::{get_line_bounds, print_error_location::PrintErrorLocation},
  },
};

impl<'a> Visit<'a> for I18NVisitor<'a> {
  fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
    const TRANSLATION_FUNCTIONS: [&str; 1] = ["t"];
    if let Some(name) = expr.callee_name() {
      self.extract_namespace(name, expr);
      if TRANSLATION_FUNCTIONS.contains(&name)
        && let Some(key) = self.extract_t_function_key(expr)
      {
        trace!("Key: {key}", key = key.italic().cyan());
        let (value, i18next_options) = self.read_t_args((expr.arguments.get(1), expr.arguments.get(2)));

        let (key, namespace) = self.get_namespace(i18next_options.as_ref(), &key);
        let has_count = match &i18next_options {
          Some(opt) => opt.get("count").is_some(),
          None => false,
        };

        let context = match &i18next_options {
          Some(opt) => {
            let context = opt.get("context");
            if let Some(Some(context_value)) = context {
              let ctx_value = context_value
                .as_array()
                .map(|val| {
                  val
                    .iter()
                    .filter_map(|val| val.as_str().map(|val| val.to_string()))
                    .collect::<_>()
                })
                .or(context_value.as_str().map(|val| val.to_string()).map(|val| vec![val]));

              if ctx_value.is_none() {
                let line = get_line_bounds(
                  self.program.source_text,
                  usize::try_from(expr.span.start).expect("span size overload"),
                  usize::try_from(expr.span.end).expect("span size overload"),
                )
                .map(|(start, end)| {
                  if start == end {
                    format!(":{start}")
                  } else {
                    format!(":{start}-{end}")
                  }
                })
                .unwrap_or_default();
                log::warn!(
                  "Unable to find the value of {key} {value:?} in {file_name}{line}",
                  key = "context".cyan(),
                  value = context_value.blue().bold(),
                  file_name = self.file_path.display().yellow(),
                  line = line.blue()
                );
                self.print_error_location(&expr.span);
              }

              ctx_value
            } else if let Some(None) = context {
              let line = get_line_bounds(
                self.program.source_text,
                usize::try_from(expr.span.start).expect("span size overload"),
                usize::try_from(expr.span.end).expect("span size overload"),
              )
              .map(|(start, end)| {
                if start == end {
                  format!(":{start}")
                } else {
                  format!(":{start}-{end}")
                }
              })
              .unwrap_or_default();
              log::warn!(
                "Unable to find the value of props {key} in {file_name}{line}",
                key = "context".cyan(),
                file_name = self.file_path.display().yellow(),
                line = line.blue()
              );
              self.print_error_location(&expr.span);
              None
            } else {
              None
            }
          }
          None => None,
        };

        self.entries.push(Entry {
          location: Location::new(
            self.file_path.to_str().unwrap().to_string(),
            usize::try_from(expr.span.start).unwrap(),
            usize::try_from(expr.span.end).unwrap(),
          ),
          key,
          value: value.and_then(|v| decode_html_entities(&v).ok()),
          namespace,
          has_count,
          i18next_options,
          context,
        });
      };
    }
    walk::walk_call_expression(self, expr);
  }

  fn visit_jsx_element(&mut self, elem: &JSXElement<'a>) {
    const COMPONENT_FUNCTIONS: [&str; 1] = ["Trans"];
    match &elem.opening_element.name {
      JSXElementName::Identifier(id) if COMPONENT_FUNCTIONS.contains(&id.name.as_ref()) => {
        trace!("Extracting data from {}", elem.opening_element.name);
        self.extract_jsx_entries(elem);
      }
      JSXElementName::IdentifierReference(id) if COMPONENT_FUNCTIONS.contains(&id.name.as_ref()) => {
        self.extract_jsx_entries(elem);
      }
      _ => {}
    };
    walk::walk_jsx_element(self, elem);
  }
}
