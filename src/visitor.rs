use std::collections::HashMap;

use log::{trace, warn};
use oxc_ast::ast::*;
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
  pub entries: Vec<Entry>,

  /// the current namespace while parsing a file
  current_namespace: Option<String>,
}

impl<'a> I18NVisitor<'a> {
  /// Creates a new [`CountASTNodes`].
  pub fn new(program: &'a Program<'a>) -> Self {
    I18NVisitor { program, current_namespace: Default::default(), entries: Default::default() }
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
    let arr =
      self.program.body.iter().find_map(|stmt| {
        if let Statement::VariableDeclaration(var) = stmt {
          var.declarations.iter().find(|v| v.id.get_identifier() == Some(&identifier.name)).and_then(|item| {
            item.init.as_ref().and_then(|init| Self::parse_string_literal_or_satisfies_expression(init))
          })
        } else {
          None
        }
      });

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

  fn get_prop_value(&self, elem: &JSXElement<'_>, attribute_name: &str) -> Option<String> {
    let collect = elem
      .opening_element
      .attributes
      .iter()
      .filter_map(|elem| match elem {
        JSXAttributeItem::Attribute(attribute) => {
          if let JSXAttributeName::Identifier(identifier) = &attribute.name {
            if identifier.name == attribute_name {
              if let Some(value) = &attribute.value {
                match value {
                  JSXAttributeValue::StringLiteral(str) => Some(str.value.to_string()),
                  JSXAttributeValue::ExpressionContainer(e) => {
                    // todo this expression will contains the required identifier
                    match &e.expression {
                      JSXExpression::StringLiteral(str) => Some(str.value.to_string()),
                      JSXExpression::Identifier(identifier) => self.find_identifier_value(identifier),
                      _ => todo!("expression container {e:?} not supported"),
                    }
                  },
                  JSXAttributeValue::Element(_) => todo!("element not supported"),
                  JSXAttributeValue::Fragment(_) => todo!("fragment not supported"),
                }
              } else {
                None
              }
            } else {
              None
            }
          } else {
            None
          }
        },
        JSXAttributeItem::SpreadAttribute(_) => todo!("warn that spread attribute is not supported"),
      })
      .collect::<Vec<_>>();

    collect.first().map(|v| v.to_string())
  }

  #[allow(clippy::ptr_arg)]
  fn elem_to_string(childs: &Vec<NodeChild>) -> String {
    childs
      .iter()
      .enumerate()
      .map(|(index, e)| match e {
        NodeChild::Text(text) => text.clone(),
        NodeChild::Js(text) => text.clone(),
        NodeChild::Tag(tag) => {
          let use_tag_name = tag.is_basic;
          let element_name = if use_tag_name { tag.name.clone() } else { format!("{}", index) };
          let children_string = tag.children.as_ref().map(Self::elem_to_string).unwrap_or_default();

          if !children_string.is_empty() || !(use_tag_name && tag.self_closing) {
            format!("<{element_name}>{children_string}</{element_name}>")
          } else {
            format!("<{element_name} />")
          }
        },
      })
      .collect::<Vec<_>>()
      .concat()
  }

  fn parse_children(childs: &oxc_allocator::Vec<JSXChild<'a>>) -> Vec<NodeChild> {
    childs
      .iter()
      .map(|child| match child {
        JSXChild::Text(text) => {
          let atom = &text.value;
          let clean_multi_line_code = clean_multi_line_code(atom);
          trace!("Text: {atom:?} -> {clean_multi_line_code:?}");
          NodeChild::Text(clean_multi_line_code)
        },
        JSXChild::Element(element) => {
          let name = if let JSXElementName::Identifier(id) = &element.opening_element.name { &id.name } else { "" };
          let is_basic = element.opening_element.attributes.len() == 0;
          let has_dynamic_children = element.children.iter().any(|child| match child {
            JSXChild::Text(_) => todo!(),
            JSXChild::Element(e) => {
              if let JSXElementName::Identifier(id) = &e.opening_element.name {
                id.name.eq("i18nIsDynamicList")
              } else {
                false
              }
            },
            _ => false,
          });
          let children = if has_dynamic_children {
            None
          } else {
            let childs = &element.children;
            Some(Self::parse_children(childs))
          };

          NodeChild::Tag(NodeTag {
            children,
            name: name.to_string(),
            is_basic,
            self_closing: element.closing_element.is_none(),
          })
        },
        JSXChild::ExpressionContainer(exp) => {
          let exp = exp.expression.as_expression().map(|exp| Self::parse_expression(exp));
          exp.unwrap_or(NodeChild::Text("".to_string()))
        },
        _ => todo!(),
      })
      .filter(|e| !e.is_empty())
      .collect::<Vec<_>>()
  }
}

enum NodeChild {
  Text(String),
  Tag(NodeTag),
  Js(String),
}

impl NodeChild {
  fn is_empty(&self) -> bool {
    match self {
      NodeChild::Text(text) => text.is_empty(),
      NodeChild::Tag(tag) => tag.children.is_none(),
      NodeChild::Js(js) => js.is_empty(),
    }
  }
}

struct NodeTag {
  children: Option<Vec<NodeChild>>,
  name: String,
  is_basic: bool,
  self_closing: bool,
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

        self.entries.push(Entry {
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

  fn visit_jsx_element(&mut self, elem: &JSXElement<'a>) {
    // println!("JSX element: {elem:?}");
    let component_functions = ["Trans"];
    let name = match &elem.opening_element.name {
      JSXElementName::Identifier(id) => &id.name,
      JSXElementName::NamespacedName(_) => todo!(),
      JSXElementName::MemberExpression(_) => todo!(),
    };
    #[allow(unused_variables)]
    if component_functions.contains(&name.as_str()) {
      let key = self.get_prop_value(elem, "i18nKey");
      let ns = self.get_prop_value(elem, "ns");
      let default_value = self.get_prop_value(elem, "defaults");
      let count = self.get_prop_value(elem, "count");
      let options = self.get_prop_value(elem, "i18n");

      trace!("Childrens: {:?}", elem.children);
      let node_as_string = {
        let content = Self::parse_children(&elem.children);
        Self::elem_to_string(&content)
      };
      trace!("Element as string: {node_as_string:?}");

      let default_value = default_value.unwrap_or(node_as_string);
      if let Some(key) = key {
        self.entries.push(Entry {
          key,
          default_value: if default_value.is_empty() { None } else { Some(default_value) },
          namespace: ns,
          // i18next_options: options,
          ..Default::default()
        });
      }
    }

    walk::walk_jsx_element(self, elem);
  }
}

impl<'a> I18NVisitor<'a> {
  fn parse_expression(exp: &Expression<'a>) -> NodeChild {
    match &exp {
      Expression::StringLiteral(str) => NodeChild::Text(str.value.to_string()),
      Expression::AssignmentExpression(e) => Self::parse_expression(&e.right),
      Expression::TSAsExpression(e) => Self::parse_expression(&e.expression),
      Expression::CallExpression(e) if e.callee.is_identifier_reference() && e.arguments.len() >= 1 => {
        Self::parse_expression(&e.callee)
      },
      Expression::ObjectExpression(e) => {
        let non_format_props = e
          .properties
          .iter()
          .filter_map(|prop| {
            if let ObjectPropertyKind::ObjectProperty(obj) = prop {
              let option = obj.key.name().map(|name| name != "format");
              if let Some(option) = option {
                if option {
                  Some(obj)
                } else {
                  None
                }
              } else {
                None
              }
            } else {
              None
            }
          })
          .collect::<Vec<_>>();
        let format_props = e.properties.iter().find(|a| {
          if let ObjectPropertyKind::ObjectProperty(obj) = a {
            obj.key.name().map(|name| name == "format").unwrap_or_default()
          } else {
            false
          }
        });
        if non_format_props.len() > 1 {
          warn!("The passed in object contained more than one variable - the object should look like {{{{ value, format }}}} where format is optional");
          return NodeChild::Text("".to_string());
        }

        let value = if let Some(format_props) = format_props {
          let text = non_format_props.first().and_then(|p| p.key.name().map(|str| str.to_string())).unwrap_or_default();
          if let ObjectPropertyKind::ObjectProperty(obj) = format_props {
            if let Some(init) = &obj.init {
              match init {
                Expression::StringLiteral(str) => Some(format!("{}, {}", text, str.value)),
                _ => {
                  warn!("The format property should be a string");
                  None
                },
              }
            } else {
              None
            }
          } else {
            None
          }
        } else {
          non_format_props.first().map(|p| p.key.name().map(|str| str.to_string())).unwrap_or_default()
        };

        NodeChild::Js(format!("{{ {} }}", value.unwrap_or_default()))
      },
      _ => NodeChild::Text("".to_string()),
    }
  }
}

fn clean_multi_line_code(text: &str) -> String {
  text.replace(|c: char| c.is_whitespace(), " ").trim().to_string()
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
    visitor.entries
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
  #[test]
  fn should_parse_jsx_with_ns_defined_in_variable() {
    let source_text = r#"
        const ns = "ns";
        const el = <Trans ns={ns} i18nKey="dialog.title">
							Reset password
						</Trans>;
						"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 1);
    let le = keys.first().unwrap();
    le.assert_eq("dialog.title", Some("ns".to_string()), Some("Reset password".to_string()));
  }

  #[test]
  fn should_parse_jsx_with_ns() {
    let source_text = r#"
        const el = <Trans ns="ns" i18nKey="dialog.title">
							Reset password
						</Trans>;
						"#;
    let keys = parse(source_text);
    assert_eq!(keys.len(), 1);
    let le = keys.first().unwrap();
    le.assert_eq("dialog.title", Some("ns".to_string()), Some("Reset password".to_string()));
  }
}
