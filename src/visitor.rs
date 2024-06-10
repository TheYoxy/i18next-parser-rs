use std::collections::HashMap;

use log::{debug, trace, warn};
use oxc_ast::{
  ast::{Argument, CallExpression, Expression, IdentifierReference, ObjectPropertyKind, Program, Statement},
  visit::walk,
  Visit,
};

#[derive(Debug, Default)]
pub struct Entry {
  pub key: String,
  pub default_value: Option<String>,
  pub namespace: Option<String>,
  pub file_paths: String,
  /// all i18next options found in the file
  pub i18next_options: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub struct I18NVisitor<'a> {
  pub program: &'a Program<'a>,
  pub keys: Vec<Entry>,

  /// the current namespace while parsing a file
  current_namespace: Option<String>,
}

impl<'a> I18NVisitor<'a> {
  /// Creates a new [`CountASTNodes`].
  pub fn new(program: &'a Program<'a>) -> Self {
    I18NVisitor { program, current_namespace: Default::default(), keys: Default::default() }
  }

  fn parse_string_literal_or_satisfies_expression(expr: &Expression<'_>) -> Option<String> {
    match expr {
      Expression::StringLiteral(str) => Some(str.value.to_string()),
      Expression::TSSatisfiesExpression(expr) => Self::parse_string_literal_or_satisfies_expression(&expr.expression),
      _ => None,
    }
  }

  /// Find the value of an identifier.
  fn find_identifier_value(&self, identifier: &oxc_allocator::Box<IdentifierReference>) -> Option<String> {
    let collect = &self
      .program
      .body
      .iter()
      .filter_map(|stmt| {
        if let Statement::VariableDeclaration(var) = stmt {
          let filtered =
            var.declarations.iter().filter(|v| v.id.get_identifier() == Some(&identifier.name)).collect::<Vec<_>>();
          filtered.first().and_then(|item| {
            item.init.as_ref().and_then(|init| Self::parse_string_literal_or_satisfies_expression(init))
          })
        } else {
          None
        }
      })
      .collect::<Vec<_>>();
    let arr = collect.first();

    arr.map(|arr| arr.to_string())
  }

  fn extract_namespace(&mut self, name: &str, expr: &CallExpression<'a>) {
    let arg = match name {
      "useTranslation" | "withTranslation" => expr.arguments.first(),
      "getFixedT" => expr.arguments.get(1),
      _ => None,
    };
    if let Some(arg) = arg {
      match arg {
        Argument::StringLiteral(str) => {
          trace!("{name:?} Arg: {str:?}");
          todo!("Handle string literal")
        },
        Argument::Identifier(identifier) => {
          let identifier = self.find_identifier_value(identifier);
          self.current_namespace = identifier;
        },
        _ => {},
      }
    }
  }

  fn parse_i18next_option(&self, expr: Option<&Argument<'_>>) -> Option<HashMap<String, String>> {
    if let Some(Argument::ObjectExpression(obj)) = expr {
      let map = obj
        .properties
        .iter()
        .filter_map(|prop| match prop {
          ObjectPropertyKind::ObjectProperty(kv) => {
            trace!("Key: {:?}", kv.key.name());
            trace!("Value: {:?}", kv.value);
            let value = match &kv.value {
              Expression::Identifier(identifier) => self.find_identifier_value(identifier),
              Expression::StringLiteral(str) => Some(str.value.to_string()),
              Expression::BooleanLiteral(bool) => Some(bool.value.to_string()),
              _ => None,
            };

            if let Some(value) = value {
              kv.key.name().map(|name| (name.to_string(), value))
            } else {
              None
            }
          },
          ObjectPropertyKind::SpreadProperty(_) => {
            trace!("Spread property");
            warn!("Unsupported spread property");
            None
          },
        })
        .collect::<HashMap<_, _>>();

      Some(map)
    } else {
      None
    }
  }
}

impl<'a> Visit<'a> for I18NVisitor<'a> {
  fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
    // println!("Call expression: {:?}", expr);
    if let Some(name) = expr.callee_name() {
      self.extract_namespace(name, expr);
      if name == "t" {
        let value = if expr.arguments.len() > 0 {
          let arg = expr.arguments.first();
          match arg {
            Some(Argument::StringLiteral(str)) => {
              trace!("t Arg: {str:?}");
              Some(str.value.to_string())
            },
            Some(Argument::TemplateLiteral(template)) => {
              trace!("t Arg: {template:?}");
              todo!("Handle template literal")
            },
            Some(Argument::BinaryExpression(bin)) => {
              trace!("t Arg: {bin:?}");
              todo!("Handle binary expression")
            },
            _ => {
              warn!("Unknown argument type: {:?}", arg);
              None
            },
          }
        } else {
          None
        };
        trace!("Value: {:?}", value);

        let arg = expr.arguments.get(1);

        let mut i18next_options = None;
        let default_value = match arg {
          Some(Argument::StringLiteral(str)) => {
            trace!("t options: {str:?}");
            Some(str.value.to_string())
          },
          _ => {
            i18next_options = self.parse_i18next_option(arg);
            i18next_options
              .clone()
              .map(|options| options.get("defaultValue").map(|value| value.to_string()).unwrap_or_default())
          },
        };
        trace!("Default value: {default_value:?}");

        // fill options if not already filled
        if i18next_options.is_none() {
          i18next_options = self.parse_i18next_option(expr.arguments.get(2));
        }

        self.keys.push(Entry {
          key: value.unwrap_or_default(),
          default_value,
          namespace: self
            .current_namespace
            .clone()
            .or(i18next_options.clone().and_then(|o| o.get("namespace").map(|v| v.to_string()))),
          i18next_options,
          ..Default::default()
        });
      };
    }
    walk::walk_call_expression(self, expr);
  }
}

#[cfg(test)]
mod tests {
  use oxc_allocator::Allocator;
  use oxc_parser::Parser;
  use oxc_span::SourceType;

  use super::*;

  impl Entry {
    fn assert_eq(&self, key: &str, namespace: Option<String>, default_value: Option<String>) {
      assert_eq!(self.key, key, "the key does not match");
      assert_eq!(self.namespace, namespace, "the namespace does not match");
      assert_eq!(self.default_value, default_value, "the default value does not match");
    }
  }

  fn parse(source_text: &str) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&program);
    visitor.visit_program(&program);
    visitor.keys
  }

  #[test]
  fn should_parse_t_with_options_and_ns_defined_in_variable() {
    let source_text = r#"
    const ns = "ns";
    const title = t("toast.title", undefined, { namespace: ns });"#;
    let keys = parse(source_text);

    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", Some("ns".to_string()), None);
  }

  #[test]
  fn should_parse_t_with_key_only() {
    let source_text = r#"const title = t("toast.title");"#;
    let keys = parse(source_text);

    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", None, None);
  }

  #[test]
  fn should_parse_t_with_options() {
    let source_text = r#"const title = t("toast.title", "default_value", {namespace: "ns"});"#;
    let keys = parse(source_text);

    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", Some("ns".to_string()), Some("default_value".to_string()));
  }

  #[test]
  fn should_parse_t_with_default_value() {
    let source_text = r#"const title = t("toast.title", "nns");"#;
    let keys = parse(source_text);

    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", None, Some("nns".to_string()));
  }

  #[test]
  fn should_parse_get_fixed_t_with_ns() {
    let source_text = r#"
      const ns = "ns";
      const t = await i18next.getFixedT(locale, ns);
      const title = t("toast.title");
    "#;

    let keys = parse(source_text);

    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", Some("ns".to_string()), None);
  }

  #[test]
  fn should_parse_t_with_default_value_and_ns_defined_in_variable() {
    let source_text = r#"
        const ns = "ns";
        const title = t("toast.title", "default title", { namespace: ns });"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", Some("ns".to_string()), Some("default title".to_string()));
  }

  #[test]
  fn should_parse_t_with_no_options() {
    let source_text = r#"const title = t("toast.title");"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", None, None);
  }

  #[test]
  fn should_parse_t_with_empty_options() {
    let source_text = r#"const title = t("toast.title", undefined, {});"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 1);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title", None, None);
  }

  #[test]
  fn should_parse_t_with_multiple_keys() {
    let source_text = r#"
        const title1 = t("toast.title1");
        const title2 = t("toast.title2");
        const title3 = t("toast.title3");"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 3);
    let el = keys.first().unwrap();
    el.assert_eq("toast.title1", None, None);
  }

  #[test]
  fn should_parse_t_with_same_key_multiple_times() {
    let source_text = r#"
        const title1 = t("toast.title");
        const title2 = t("toast.title");
        const title3 = t("toast.title");"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 3);
    for el in keys {
      el.assert_eq("toast.title", None, None);
    }
  }
}
