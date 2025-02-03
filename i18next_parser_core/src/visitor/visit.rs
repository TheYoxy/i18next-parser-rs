use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::{
  ast::{Argument, CallExpression, ChainExpression, JSXElement, JSXElementName},
  visit::walk,
  Visit,
};
use oxc_span::GetSpan;

use crate::{visitor::I18NVisitor, Entry};

#[cfg(debug_assertions)]
fn print_error_location(span: &oxc_span::Span, file_path: &std::path::PathBuf) {
  use log::error;
  let content = std::fs::read_to_string(file_path).unwrap();
  let (start, remaining) = content.split_at(span.start.try_into().unwrap());
  let (content, end) = remaining.split_at(span.size().try_into().unwrap());
  let (previous, previous_content) = start.split_at(start.rfind('\n').unwrap());
  let (_, last_line) = previous.split_at(previous.rfind('\n').unwrap());
  let (next_content, next) = end.split_at(end.rfind('\n').unwrap());
  let (next_line, _) = next.split_at(next.rfind('\n').unwrap());
  let line = start.chars().fold(0, |i, c| if c == '\n' { i + 1 } else { i });

  error!("Location: ");
  error!("{line}: {last_line}", last_line = last_line.replace('\n', ""));
  let line = line + 1;
  error!(
    "{line}: {previous_content}{content}{next_content}",
    previous_content = previous_content.replace('\n', ""),
    content = content.italic().red().underline(),
    next_content = next_content.replace('\n', ""),
  );
  let line = line + 1;
  error!("{line}: {next_line}", next_line = next_line.replace('\n', ""));
}

impl<'a> Visit<'a> for I18NVisitor<'a> {
  fn visit_chain_expression(&mut self, it: &ChainExpression<'a>) {
    log::info!("Chain expression: {it:?}", it = it);
    walk::walk_chain_expression(self, it);
  }

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
              print_error_location(&template.span, &self.file_path);
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
              print_error_location(&bin.span, &self.file_path);
              todo!("Handle binary expression")
            }
            #[cfg(not(debug_assertions))]
            {
              warn!("Binary expression are not supported for now");
              None
            }
          },

          Some(Argument::CallExpression(call)) => {
            trace!("t Arg: {:?}", call.bright_black().italic());
            trace!("t callee: {:?}", call.callee);
            call.common_js_require().inspect(|req| trace!("t require: {}", req.value.to_string()));
            call.callee_name().inspect(|name| trace!("t callee name: {}", name));
            trace!("t arguments: {:?}", call.arguments);
            for (idx, arg) in call.arguments.iter().enumerate() {
              trace!("t argument: {idx} {:?}", arg);
            }
            #[cfg(debug_assertions)]
            {
              print_error_location(&call.span, &self.file_path);

              todo!("Handle call expression")
            }
            #[cfg(not(debug_assertions))]
            {
              warn!("Call expression are not supported for now");
              None
            }
          },
          Some(arg) => {
            #[cfg(debug_assertions)]
            {
              log::error!("Unknown argument type found in [{}]: {arg:?}", self.file_path.display().yellow());
              print_error_location(&arg.span(), &self.file_path);

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
