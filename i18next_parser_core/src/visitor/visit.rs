use color_eyre::owo_colors::OwoColorize;
use log::trace;
use oxc_ast::ast::{CallExpression, JSXElement, JSXElementName};
use oxc_ast_visit::{Visit, walk};

use crate::{
  Entry,
  visitor::{I18NVisitor, entry::Location},
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
          Some(opt) => opt.get("context").cloned().unwrap_or(None).map(|v| vec![v]),
          None => None,
        };

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
      },
      JSXElementName::IdentifierReference(id) if COMPONENT_FUNCTIONS.contains(&id.name.as_ref()) => {
        self.extract_jsx_entries(elem);
      },
      _ => {},
    };
    walk::walk_jsx_element(self, elem);
  }
}
