use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::ast::{Expression, JSXChild, JSXElementName, ObjectExpression, ObjectPropertyKind};

use crate::{
  clean_multi_line_code,
  visitor::{
    node_child::{NodeChild, NodeTag},
    traits::oxc_custom_parser::OxcCustomParser,
    I18NVisitor,
    I18NextOptions,
  },
  IsEmpty,
};

impl<'a> I18NVisitor<'a> {
  pub(super) fn parse_option_and_default_value(
    &self,
    obj: &oxc_allocator::Box<'_, ObjectExpression<'_>>,
  ) -> (I18NextOptions, Option<String>) {
    let i18next_options = self.parse_i18next_option(obj);
    let default_value = i18next_options.get("defaultValue").cloned().flatten();
    if let Some(value) = i18next_options.get("defaultValue") {
      trace!("translation value found in i18next options: {value:?}");
    }
    (i18next_options, default_value)
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
  pub(super) fn parse_i18next_option(&self, obj: &oxc_allocator::Box<ObjectExpression>) -> I18NextOptions {
    let len = obj.properties.len();
    trace!("Parsing {len} properties for i18next options", len = len.blue());

    obj
      .properties
      .iter()
      .filter_map(|prop| {
        match prop {
          ObjectPropertyKind::ObjectProperty(kv) => {
            let name = kv.key.name().unwrap();

            match name.to_string().as_str() {
              "defaultValue" | "count" | "namespace" => {
                let value = self.parse_expression_as_string(&kv.value);
                kv.key.name().map(|name| (name.to_string(), value))
              },
              "ns" => {
                let value = self.parse_expression_as_string(&kv.value);
                Some(("namespace".into(), value))
              },
              _ => {
                debug!("Couldn't parse {}", name.yellow());
                None
              },
            }
          },
          ObjectPropertyKind::SpreadProperty(prop) => {
            #[cfg(debug_assertions)]
            {
              warn!("{} Unsupported spread property", "[Parse_i18next_option]".red().bold());
              self.print_error_location(&prop.span);
              panic!("Spread property is not supported in i18next options");
            }
            #[cfg(not(debug_assertions))]
            None
          },
        }
      })
      .collect::<I18NextOptions>()
  }

  pub(super) fn parse_jsx_children(childs: &oxc_allocator::Vec<JSXChild<'a>>) -> Vec<NodeChild> {
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
            let is_basic = element.opening_element.attributes.is_empty();
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
              Some(Self::parse_jsx_children(childs))
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

  pub(super) fn parse_expression_child(exp: &Expression<'a>) -> NodeChild {
    match &exp {
      Expression::StringLiteral(str) => NodeChild::Text(str.value.to_string()),
      Expression::AssignmentExpression(e) => Self::parse_expression_child(&e.right),
      Expression::TSAsExpression(e) => Self::parse_expression_child(&e.expression),
      Expression::CallExpression(e) if e.callee.is_identifier_reference() && !e.arguments.is_empty() => {
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
            match &obj.value {
              Expression::StringLiteral(str) => Some(format!("{}, {}", text, str.value)),
              _ => {
                warn!("The format property should be a string literal");
                None
              },
            }
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
}
