use std::{collections::HashMap, path::PathBuf};

use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::ast::{
  Argument, CallExpression, Expression, IdentifierReference, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
  JSXChild, JSXElement, JSXElementName, JSXExpression, ObjectExpression, ObjectPropertyKind, Program, Statement,
};
use serde_json::Value;
use tracing::span;

use crate::{
  clean_multi_line_code,
  visitor::node_child::{NodeChild, NodeTag},
  Entry, IsEmpty,
};

/// This type alias represents the options for i18next.
/// It is a HashMap where the key is a String representing the option name,
/// and the value is an Option<`String`> representing the option value.
pub type I18NextOptions = HashMap<String, Option<String>>;

/// This struct represents the options for the I18NVisitor.
///
/// # Fields
///
/// * `trans_keep_basic_html_nodes_for` - An optional vector of strings representing the basic HTML nodes to be kept for translation.
#[derive(Debug, Default)]
pub struct VisitorOptions {
  pub trans_keep_basic_html_nodes_for: Option<Vec<String>>,
}

/// This struct represents the I18NVisitor which is used to parse the AST and extract the i18n keys.
///
/// # Fields
///
/// * `program` - The program to be parsed.
/// * `entries` - A vector of entries in the i18n system.
/// * `options` - The options for the I18NVisitor.
/// * `current_namespace` - The current namespace while parsing a file.
#[derive(Debug)]
pub struct I18NVisitor<'a> {
  /// the program to be parsed
  pub program: &'a Program<'a>,
  /// the file name of the file being parsed
  pub file_path: PathBuf,
  /// the entries in the i18n system
  pub entries: Vec<Entry>,
  /// the options for the I18NVisitor
  pub options: VisitorOptions,
  /// the current namespace while parsing a file
  pub(super) current_namespace: Option<String>,
}

/// The visitor implementation that will search for translations inside javascript code
impl<'a> I18NVisitor<'a> {
  /// Creates a new \[`CountASTNodes`\].
  pub fn new<Path: Into<PathBuf>>(program: &'a Program<'a>, file_path: Path) -> Self {
    I18NVisitor {
      program,
      file_path: file_path.into(),
      entries: Default::default(),
      options: Default::default(),
      current_namespace: Default::default(),
    }
  }

  /// Parse an expression to find its value
  ///
  /// # Arguments
  ///
  /// * `expr` - The expression to parse
  ///
  /// # Returns
  ///
  /// An optional value representing the value of the expression
  fn parse_expression(&self, expr: &Expression<'_>) -> Option<Value> {
    use serde_json::json;
    trace!("Parsing expression: {:?}", expr.bright_black().italic());

    match expr {
      Expression::StringLiteral(str) => Some(json!(str.value.to_string())),
      Expression::NumericLiteral(num) => Some(json!(num.value.to_string())),
      Expression::BooleanLiteral(bool) => Some(json!(bool.value.to_string())),
      // Expression::Identifier(identifier) => self.find_identifier_value_as_string(identifier),
      // Expression::TSSatisfiesExpression(expr) => self.parse_expression_as_string(&expr.expression),
      _ => {
        debug!("Unsupported expression: {expr:?}");
        None
      },
    }
  }

  /// Parse an expression to find its value
  ///
  /// # Arguments
  ///
  /// * `expr` - The expression to parse
  ///
  /// # Returns
  ///
  /// An optional value representing the value of the expression
  fn parse_expression_as_string(&self, expr: &Expression<'_>) -> Option<String> {
    trace!("Parsing expression: {:?}", expr.bright_black().italic());

    match expr {
      Expression::StaticMemberExpression(expression) => self.parse_expression_as_string(&expression.object),
      Expression::Identifier(identifier) => {
        trace!("Looking for identifier value from expression");
        self.find_identifier_value_as_string(identifier)
      },
      Expression::TSSatisfiesExpression(expr) => {
        trace!("Looking for identifier value from expression");
        self.parse_expression_as_string(&expr.expression)
      },
      Expression::StringLiteral(str) => Some(str.value.to_string()),
      Expression::NumericLiteral(num) => Some(num.value.to_string()),
      Expression::BooleanLiteral(bool) => Some(bool.value.to_string()),
      _ => {
        warn!("Unsupported expression: {expr:?}");
        None
      },
    }
  }

  /// Find the value of an identifier.
  ///
  /// # Arguments
  ///
  /// * `identifier` - The identifier to find the value for
  ///
  /// # Returns
  ///
  /// An optional value representing the value of the identifier
  fn find_identifier_value(&self, identifier: &oxc_allocator::Box<IdentifierReference>) -> Option<Value> {
    let arr = self.program.body.iter().find_map(|stmt| {
      if let Statement::VariableDeclaration(var) = stmt {
        var
          .declarations
          .iter()
          .find(|v| v.id.get_identifier() == Some(identifier.name.clone()))
          .and_then(|item| item.init.as_ref())
          .and_then(|init| self.parse_expression(init))
      } else {
        None
      }
    });

    if arr.is_none() {
      warn!(
        "Cannot find identifier value in {} for {name} {identifier:?}",
        self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
    }

    arr
  }

  /// Find the value of an identifier as a string
  ///
  /// # Arguments
  ///
  /// * `identifier` - The identifier to find the value for
  ///
  /// # Returns
  ///
  /// An optional string representing the value of the identifier
  fn find_identifier_value_as_string(&self, identifier: &oxc_allocator::Box<IdentifierReference>) -> Option<String> {
    let arr = self.program.body.iter().find_map(|stmt| {
      if let Statement::VariableDeclaration(var) = stmt {
        var
          .declarations
          .iter()
          .find(|v| v.id.get_identifier() == Some(identifier.name.clone()))
          .and_then(|item| item.init.as_ref())
          .and_then(|init| {
            trace!("Looking for expression value from {:?}", init.bright_black().italic());
            self.parse_expression_as_string(init)
          })
      } else {
        None
      }
    });

    if arr.is_none() {
      debug!(
        "Cannot find identifier str value in {} for {name} {identifier:?}",
        self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
    }

    arr
  }

  /// Extract the namespace from the i18next function
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the function
  /// * `expr` - The call expression
  ///
  /// # Returns
  ///
  /// The namespace found in the function
  pub(super) fn extract_namespace(&mut self, name: &str, expr: &CallExpression<'a>) {
    let arg = match name {
      "useTranslation" | "withTranslation" => expr.arguments.first(),
      "getFixedT" => expr.arguments.get(1),
      _ => None,
    };
    if let Some(arg) = arg {
      match arg {
        Argument::StringLiteral(str) => {
          let value = str.value.to_string();
          trace!("{} Arg: {}", name.cyan(), value.blue());
          self.current_namespace = if value.is_empty() { None } else { Some(value) };
        },
        Argument::Identifier(identifier) => {
          trace!("Looking for namespace {} value from identifier", name.cyan());
          let identifier = self.find_identifier_value_as_string(identifier);
          self.current_namespace = identifier;
        },
        Argument::TSAsExpression(expression) => {
          trace!("Looking for namespace {} value from `As` expression", name.cyan());
          self.current_namespace = self.parse_expression_as_string(&expression.expression);
        },
        _ => {
          warn!("Unsupported argument for {name} {arg:?}");
        },
      }
    }
  }

  /// Parse the i18next options
  ///
  /// # Arguments
  ///
  /// * `obj` - The object expression to parse
  ///
  /// # Returns
  ///
  /// The i18next options found in the object
  fn parse_i18next_option(&self, obj: &oxc_allocator::Box<ObjectExpression>) -> I18NextOptions {
    use color_eyre::owo_colors::OwoColorize;

    let len = obj.properties.len();
    trace!("Parsing {len} properties for i18next options", len = len.blue());

    obj
      .properties
      .iter()
      .enumerate()
      .filter_map(|(idx, prop)| {
        match prop {
          ObjectPropertyKind::ObjectProperty(kv) => {
            let name = kv.key.name().unwrap();

            let parse = || {
              trace!(
                "Parsing key {key} {idx} from {path}",
                key = name.blue(),
                idx = idx.cyan(),
                path = self.file_path.display().yellow()
              );
              let value = self.parse_expression_as_string(&kv.value);
              trace!(
                "Parsed {key}: {parsed_value:?} <- {value:?}",
                key = name.blue(),
                parsed_value = value.yellow(),
                value = kv.value
              );
              value
            };

            match name.to_string().as_str() {
              "defaultValue" | "count" | "namespace" => {
                let value = parse();
                kv.key.name().map(|name| (name.to_string(), value))
              },
              "ns" => {
                let value = parse();
                Some(("namespace".into(), value))
              },
              _ => {
                debug!("Couldn't parse {}", name.yellow());
                None
              },
            }
          },
          ObjectPropertyKind::SpreadProperty(_) => {
            if cfg!(debug_assertions) {
              warn!("Unsupported spread property in {file}", file = self.file_path.display().yellow());
            }
            None
          },
        }
      })
      .collect::<I18NextOptions>()
  }

  /// Check if a prop exists in a JSX element
  ///
  /// # Arguments
  ///
  /// * `elem` - The JSX element to check
  /// * `attribute_name` - The name of the attribute to check
  ///
  /// # Returns
  ///
  /// A boolean indicating whether the prop exists
  pub(super) fn has_prop(&self, elem: &JSXElement<'_>, attribute_name: &str) -> bool {
    elem.opening_element.attributes.iter().any(|elem| {
      match elem {
        JSXAttributeItem::Attribute(attribute) => {
          if let JSXAttributeName::Identifier(identifier) = &attribute.name {
            if identifier.name == attribute_name {
              if let Some(value) = &attribute.value {
                match value {
                  JSXAttributeValue::StringLiteral(_) => true,
                  JSXAttributeValue::ExpressionContainer(_) => true,
                  JSXAttributeValue::Element(_) => todo!("element not supported"),
                  JSXAttributeValue::Fragment(_) => todo!("fragment not supported"),
                }
              } else {
                false
              }
            } else {
              false
            }
          } else {
            false
          }
        },
        JSXAttributeItem::SpreadAttribute(_) => todo!("warn that spread attribute is not supported"),
      }
    })
  }

  /// Get the value of a prop in a JSX element
  ///
  /// # Arguments
  ///
  /// * `elem` - The JSX element to get the prop value from
  /// * `attribute_name` - The name of the attribute to get the value for
  ///
  /// # Returns
  ///
  /// The value of the prop
  pub(super) fn get_prop_value(&self, elem: &JSXElement<'_>, attribute_name: &str) -> Option<String> {
    _ = span!(tracing::Level::TRACE, "get_prop_value", attribute_name = attribute_name).enter();
    elem
      .opening_element
      .attributes
      .iter()
      .filter_map(|elem| {
        match elem {
          JSXAttributeItem::Attribute(attribute) => {
            if let JSXAttributeName::Identifier(identifier) = &attribute.name {
              if identifier.name == attribute_name {
                if let Some(value) = &attribute.value {
                  trace!("Value: {attribute_name} {value:?}");
                  match value {
                    JSXAttributeValue::StringLiteral(str) => Some(str.value.to_string()),
                    JSXAttributeValue::ExpressionContainer(e) => {
                      // todo this expression will contains the required identifier
                      match &e.expression {
                        JSXExpression::StringLiteral(str) => Some(str.value.to_string()),
                        JSXExpression::Identifier(identifier) => {
                          trace!("Looking for identifier value for prop");
                          self.find_identifier_value_as_string(identifier)
                        },
                        JSXExpression::NumericLiteral(num) => Some(num.value.to_string()),
                        JSXExpression::StaticMemberExpression(expression) => {
                          self.parse_expression_as_string(&expression.object)
                        },
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
        }
      })
      .next()
      .map(|v| v.to_string())
  }

  /// Convert the children of a tag to a string
  pub(super) fn elem_to_string(&self, childs: &[NodeChild]) -> String {
    childs
      .iter()
      .enumerate()
      .map(|(index, e)| {
        match e {
          NodeChild::Text(text) => text.clone(),
          NodeChild::Js(text) => text.clone(),
          NodeChild::Tag(tag) => {
            let tag_name = &tag.name;
            let use_tag_name = tag.is_basic
              && self.options.trans_keep_basic_html_nodes_for.as_ref().is_some_and(|nodes| nodes.contains(tag_name));
            let element_name = if use_tag_name { tag_name } else { &format!("{}", index) };
            let children_string = tag.children.as_ref().map(|v| self.elem_to_string(v)).unwrap_or_default();
            if !(children_string.is_empty() && use_tag_name && tag.self_closing) {
              format!("<{element_name}>{children_string}</{element_name}>")
            } else {
              format!("<{element_name} />")
            }
          },
        }
      })
      .collect::<Vec<_>>()
      .concat()
  }

  pub(super) fn parse_children(childs: &oxc_allocator::Vec<JSXChild<'a>>) -> Vec<NodeChild> {
    childs
      .iter()
      .map(|child| {
        match child {
          JSXChild::Text(text) => {
            let atom = &text.value;
            let clean_multi_line_code = clean_multi_line_code(atom);
            trace!("Text: {atom:?} -> {clean_multi_line_code:?}");
            NodeChild::Text(clean_multi_line_code)
          },
          JSXChild::Element(element) => {
            let name = if let JSXElementName::Identifier(id) = &element.opening_element.name { &id.name } else { "" };
            let is_basic = element.opening_element.attributes.len() == 0;
            let has_dynamic_children = element.children.iter().any(|child| {
              if let JSXChild::Element(e) = child {
                if let JSXElementName::Identifier(id) = &e.opening_element.name {
                  id.name.eq("i18nIsDynamicList")
                } else {
                  false
                }
              } else {
                false
              }
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
            let exp = exp.expression.as_expression().map(Self::parse_expression_child);
            exp.unwrap_or(NodeChild::Text("".to_string()))
          },
          _ => todo!(),
        }
      })
      .filter(|e| !e.is_empty())
      .collect::<Vec<_>>()
  }

  fn parse_expression_child(exp: &Expression<'a>) -> NodeChild {
    match &exp {
      Expression::StringLiteral(str) => NodeChild::Text(str.value.to_string()),
      Expression::AssignmentExpression(e) => Self::parse_expression_child(&e.right),
      Expression::TSAsExpression(e) => Self::parse_expression_child(&e.expression),
      Expression::CallExpression(e) if e.callee.is_identifier_reference() && e.arguments.len() >= 1 => {
        Self::parse_expression_child(&e.callee)
      },
      Expression::ObjectExpression(e) => {
        let non_format_props = e
          .properties
          .iter()
          .filter_map(|prop| {
            if let ObjectPropertyKind::ObjectProperty(obj) = prop {
              obj.key.name().map(|name| name != "format").and_then(|o| if o { Some(obj) } else { None })
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
            obj.init.as_ref().and_then(|init| {
              match &init {
                Expression::StringLiteral(str) => Some(format!("{}, {}", text, str.value)),
                _ => {
                  warn!("The format property should be a string literal");
                  None
                },
              }
            })
          } else {
            None
          }
        } else {
          non_format_props.first().map(|p| p.key.name().map(|str| str.to_string())).unwrap_or_default()
        };

        NodeChild::Js(format!("{{{{{}}}}}", value.unwrap_or_default()))
      },
      _ => NodeChild::Text("".to_string()),
    }
  }

  pub(super) fn read_t_args(
    &mut self,
    args: (Option<&Argument<'a>>, Option<&Argument<'a>>),
  ) -> (Option<String>, Option<I18NextOptions>) {
    debug!("Reading t arguments: {:?} - {:?}", args.0.bright_black().italic(), args.1.bright_black().italic());

    match args {
      (Some(Argument::StringLiteral(str)), Some(Argument::ObjectExpression(obj))) => {
        let value = str.value.to_string();
        trace!("translation value defined as string literal: {}", value.cyan());
        let (i18next_options, default_value) = self.parse_option_and_default_value(obj);

        let value = if value.is_empty() { default_value } else { Some(value) };
        (value, Some(i18next_options))
      },
      (Some(Argument::StringLiteral(str)), None) => {
        let value = str.value.to_string();
        trace!("translation value defined as string literal: {}", value.cyan());
        (Some(value), None)
      },
      (Some(Argument::ObjectExpression(obj)), None) => {
        trace!("settings provided as 2nd argument {:?}", obj.bright_black().italic());
        let (i18next_options, default_value) = self.parse_option_and_default_value(obj);

        (default_value, Some(i18next_options))
      },
      (None, Some(Argument::ObjectExpression(obj))) => {
        trace!("settings provided as 3rd argument without 2nd argument");
        let (i18next_options, default_value) = self.parse_option_and_default_value(obj);

        (default_value, Some(i18next_options))
      },
      (Some(Argument::Identifier(identifier)), Some(Argument::ObjectExpression(obj))) => {
        debug!("looking for identifier value in t");
        let value = self.find_identifier_value(identifier);
        let (i18next_options, default_value) = self.parse_option_and_default_value(obj);
        if value.is_none() {
          (default_value, Some(i18next_options))
        } else {
          todo!("Handle identifier {identifier:?}")
        }
      },
      (None, None) => (None, None),
      (arg_1, arg_2) => {
        warn!("Unknown argument combination type: {arg_1:?} {arg_2:?}");
        todo!("Handle argument {arg_1:?} {arg_2:?}")
      },
    }
  }

  fn parse_option_and_default_value(
    &mut self,
    obj: &oxc_allocator::Box<'_, ObjectExpression<'_>>,
  ) -> (I18NextOptions, Option<String>) {
    let i18next_options = self.parse_i18next_option(obj);
    let default_value = i18next_options.get("defaultValue").cloned().flatten();
    if let Some(value) = i18next_options.get("defaultValue") {
      trace!("translation value found in i18next options: {value:?}");
    }
    (i18next_options, default_value)
  }
}

#[cfg(test)]
mod tests {
  use oxc_allocator::Allocator;
  use oxc_ast::Visit;
  use oxc_parser::Parser;
  use oxc_span::SourceType;

  use super::*;

  fn parse(source_text: &str) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&program, "file.tsx");
    visitor.visit_program(&program);
    visitor.entries
  }

  fn parse_with_options(source_text: &str) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&program, "file.tsx");
    visitor.options.trans_keep_basic_html_nodes_for =
      Some(vec!["br".to_string(), "strong".to_string(), "i".to_string(), "p".to_string()]);
    visitor.visit_program(&program);
    visitor.entries
  }

  mod t_function {
    use super::*;

    #[test_log::test]
    fn should_parse_t_with_options_and_ns_defined_in_variable() {
      // language=javascript
      let source_text = "const ns = 'ns'; const title = t('toast.title', undefined, {namespace: ns});";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
    }

    #[test_log::test]
    fn should_parse_t_with_key_only() {
      // language=javascript
      let source_text = "const title = t('toast.title');";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
    }

    #[test_log::test]
    fn should_parse_t_with_options() {
      // language=javascript
      let source_text = "const title = t('toast.title', 'default_value', {namespace: 'ns'});";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("toast.title", "default_value", "ns")]);
    }

    #[test_log::test]
    fn should_parse_t_with_default_value() {
      // language=javascript
      let source_text = "const title = t('toast.title', 'nns');";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_value("toast.title", "nns")]);
    }

    #[test_log::test]
    fn should_parse_get_fixed_t_with_ns() {
      // language=javascript
      let source_text =
        "const ns = 'ns'; const t = await i18next.getFixedT(locale, ns); const title = t('toast.title'); ";

      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
    }

    #[test_log::test]
    fn should_parse_t_with_default_value_and_ns_defined_in_variable() {
      // language=javascript
      let source_text = "const ns = 'ns'; const title = t('toast.title', 'default title', { namespace: ns });";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("toast.title", "default title", "ns")]);
    }

    #[test_log::test]
    fn should_parse_t_with_no_options() {
      // language=javascript
      let source_text = "const title = t('toast.title');";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
    }

    #[test_log::test]
    fn should_parse_t_with_empty_options() {
      // language=javascript
      let source_text = "const title = t('toast.title', undefined, {});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
    }

    #[test_log::test]
    fn should_parse_t_with_multiple_keys() {
      // language=javascript
      let source_text =
        "const title1 = t('toast.title1'); const title2 = t('toast.title2'); const title3 = t('toast.title3');";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 3);
      assert_eq!(keys, vec![Entry::empty("toast.title1"), Entry::empty("toast.title2"), Entry::empty("toast.title3")]);
    }

    #[test_log::test]
    fn should_parse_t_with_same_key_multiple_times() {
      // language=javascript
      let source_text =
        "const title1 = t('toast.title'); const title2 = t('toast.title'); const title3 = t('toast.title');";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 3);
      assert_eq!(keys, vec![Entry::empty("toast.title"), Entry::empty("toast.title"), Entry::empty("toast.title")]);
    }

    #[test_log::test]
    fn should_parse_t_with_value() {
      // language=javascript
      let source_text = "const title = t('toast.title', {defaultValue: 'Attempt {{num}}', num: 0});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_value("toast.title", "Attempt {{num}}")]);
    }

    #[test_log::test]
    fn should_parse_t_with_count_literal_spread() {
      // language=javascript
      let source_text = "const count = 1; const title = t('toast.title', undefined, { count });";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
      let el = keys.first().unwrap();
      assert!(el.has_count);
    }

    #[test_log::test]
    fn should_parse_t_with_count_literal() {
      // language=javascript
      let source_text = "const count = 1; const title = t('toast.title', undefined, {count: count});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
      let el = keys.first().unwrap();
      assert!(el.has_count);
    }

    #[test_log::test]
    fn should_parse_t_with_count_numeric() {
      // language=javascript
      let source_text = "const title = t('toast.title', undefined, {count: 1});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
      let el = keys.first().unwrap();
      assert!(el.has_count);
    }

    #[test_log::test]
    fn should_parse_t_with_count_arg() {
      // language=javascript
      let source_text = "const title = (count: number) => t('toast.title', undefined, {count: count});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
      let el = keys.first().unwrap();
      assert!(el.has_count);
    }

    #[test_log::test]
    fn should_parse_t_with_count_arg_spread() {
      // language=javascript
      let source_text = "const title = (count: number) => t('toast.title', undefined, {count});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::empty("toast.title")]);
      let el = keys.first().unwrap();
      assert!(el.has_count);
    }

    #[test_log::test]
    fn should_parse_t_without_default_value_and_namespace() {
      // language=javascript
      let source_text = "const title = t('toast.title', {ns: 'namespace'});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
    }

    #[test_log::test]
    fn should_parse_t_with_default_value_and_namespace() {
      // language=javascript
      let source_text = "const title = t('toast.title', 'nns', {ns: 'namespace'});";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("toast.title", "nns", "namespace")]);
    }
  }

  mod translation_component {
    use super::*;

    #[test_log::test]
    fn should_extract_keys_from_render_props() {
      // language=javascript
      let source_text = "<Translation>{(t) => <>{t('first', 'Main')}{t('second')}</>}</Translation>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 2);
      assert_eq!(keys, vec![Entry::new_with_value("first", "Main"), Entry::empty("second")]);
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_extract_ns_from_translation_with_render_prop() {
      // language=javascript
      let source_text = "<Translation ns='foo'>{(t) => t('first')}</Translation>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("first", "foo")]);
    }
  }

  mod trans_component {
    use super::*;

    #[test_log::test]
    fn should_parse_jsx_with_ns_defined_in_variable() {
      // language=javascript
      let source_text = "const ns = 'ns'; const el = <Trans ns={ns} i18nKey='dialog.title'>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_ns() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_template_translated() {
      // language=javascript
      let source_text = "const Comp = () => <i>Reset password</i>; const el = <Trans ns='ns' i18nKey='dialog.title'><Comp>Reset password</Comp></Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "<0>Reset password</0>", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_nested_template() {
      // language=javascript
      let source_text =
        "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password {{attempt}}</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password {{attempt}}", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_nested_template_object() {
      // language=javascript
      let source_text = "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password {{ attempt: attempt + 1 }}</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password {{attempt}}", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_nested_template_object_and_text_after() {
      // language=javascript
      let source_text = "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Attempt {{ attempt: attempt + 1 }} on 10</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Attempt {{attempt}} on 10", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_self_closing_element() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password<br /></Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password<1></1>", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_template_removed_when_unspecified() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "<0>Reset password</0>", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_with_template_kept() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trans>;";
      let keys = parse_with_options(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "<i>Reset password</i>", "ns")]);
    }

    #[test_log::test]
    fn should_parse_jsx_and_return_nothing_on_bad_components() {
      // language=javascript
      let source_text = "const el = <Trad ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trad>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_identifier() {
      // language=javascript
      let source_text =
        "const count = 2; const el = <Trans ns='ns' i18nKey='dialog.title' count={count}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_numeral() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title' count={2}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_double_reference() {
      // language=javascript
      let source_text =
        "const a = 2; const b = a; const el = <Trans ns='ns' i18nKey='dialog.title' count={b}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);

      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_from_arg() {
      // language=javascript
      let source_text =
        "const el = (count: number) => <Trans ns='ns' i18nKey='dialog.title' count={count}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);

      let le = keys.first().unwrap();
      assert!(le.has_count);
    }
  }
}
