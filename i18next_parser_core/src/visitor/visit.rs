use color_eyre::owo_colors::OwoColorize;
use log::{debug, error, trace, warn};
use oxc_ast::{
  ast::{Argument, CallExpression, JSXElement, JSXElementName},
  visit::walk,
  Visit,
};
use oxc_span::GetSpan;

use crate::{visitor::I18NVisitor, Entry};

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
            if cfg!(debug_assertions) {
              trace!("t Arg: {:?}", template.bright_black().italic());
              trace!("t quasis: {:?}", template.quasis);
              trace!("t expressions: {:?}", template.expressions);
              todo!("Handle template literal")
            } else {
              warn!("Template literal are not supported for now");
              None
            }
          },
          Some(Argument::BinaryExpression(bin)) => {
            if cfg!(debug_assertions) {
              trace!("t Arg: {:?}", bin.bright_black().italic());
              todo!("Handle binary expression")
            } else {
              warn!("Binary expression are not supported for now");
              None
            }
          },

          Some(Argument::CallExpression(call)) => {
            if cfg!(debug_assertions) {
              trace!("t Arg: {:?}", call.bright_black().italic());
              trace!("t callee: {:?}", call.callee);
              call.common_js_require().inspect(|req| trace!("t require: {}", req.value.to_string()));
              call.callee_name().inspect(|name| trace!("t callee name: {}", name));
              trace!("t arguments: {:?}", call.arguments);
              for (idx, arg) in call.arguments.iter().enumerate() {
                trace!("t argument: {idx} {:?}", arg);
              }

              todo!("Handle call expression")
            } else {
              warn!("Call expression are not supported for now");
              None
            }
          },
          Some(arg) => {
            if cfg!(debug_assertions) {
              error!("Unknown argument type found in [{}]: {arg:?}", self.file_path.display().yellow());
              todo!("Handle argument {arg:?} in {}", self.file_path.display().yellow())
            } else {
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

          self.entries.push(Entry { key, value, namespace, has_count, i18next_options });
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
