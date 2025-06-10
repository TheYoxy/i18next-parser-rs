use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::ast::{
  BindingPattern,
  BindingPatternKind,
  Declaration,
  Expression,
  IdentifierReference,
  ImportDeclarationSpecifier,
  JSXChild,
  JSXElementName,
  ModuleDeclaration,
  ObjectExpression,
  ObjectPropertyKind,
  Statement,
  TSLiteral,
  TSSignature,
  TSType,
  TSTypeAnnotation,
  TSTypeName,
  TSUnionType,
};
use oxc_span::GetSpan;
use serde_json::Value;

use crate::{
  clean_multi_line_code,
  helper::SerdeHelper,
  visitor::{
    node_child::{NodeChild, NodeTag},
    I18NVisitor,
    I18NextOptions,
  },
  IsEmpty,
};

impl<'a> I18NVisitor<'a> {
  /// Parse an expression to find its value
  ///
  /// # Arguments
  ///
  /// * `expr` - The expression to parse
  ///
  /// # Returns
  ///
  /// An optional value representing the value of the expression
  pub(super) fn parse_expression_to_serde_value(&self, expr: &Expression<'_>) -> Option<Value> {
    use serde_json::json;
    trace!("Parsing expression: {:?}", expr.bright_black().italic());

    match expr {
      Expression::StringLiteral(str) => Some(json!(str.value.to_string())),
      Expression::NumericLiteral(num) => Some(json!(num.value.to_string())),
      Expression::BooleanLiteral(bool) => Some(json!(bool.value.to_string())),
      Expression::ArrayExpression(arr) => {
        Some(Value::Array(
          arr
            .elements
            .iter()
            .map(|e| {
              if let Some(expession) = e.as_expression() {
                self.parse_expression_to_serde_value(expession).unwrap_or(Value::Null)
              } else {
                Value::Null
              }
            })
            .collect(),
        ))
      },
      Expression::ObjectExpression(obj) => {
        Some(Value::Object(
          obj
            .properties
            .iter()
            .filter_map(|prop| {
              if let ObjectPropertyKind::ObjectProperty(kv) = prop {
                let key = kv.key.name().unwrap_or_default().to_string();
                let value = self.parse_expression_to_serde_value(&kv.value);
                Some((key, value.unwrap_or(Value::Null)))
              } else {
                None
              }
            })
            .collect(),
        ))
      },
      Expression::Identifier(identifier) => self.find_identifier_value_as_serde(identifier),
      Expression::TSSatisfiesExpression(expr) => self.parse_expression_to_serde_value(&expr.expression),
      Expression::TSAsExpression(expression) => self.parse_expression_to_serde_value(&expression.expression),
      _ => {
        self.print_error_location(&expr.span());
        debug!("{} Unsupported expression: {expr:?}", "[Parse_expression]".red().bold());
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
  pub(super) fn parse_expression_as_string(&self, expr: &Expression<'_>) -> Option<String> {
    self.parse_expression_to_serde_value(expr).value_to_string()
  }

  pub(super) fn parse_type_annotation_as_vec_str(
    &self,
    type_annotation: &oxc_allocator::Box<'_, TSTypeAnnotation<'_>>,
  ) -> Option<Vec<String>> {
    match &type_annotation.type_annotation {
      TSType::TSUnionType(union_type) => get_string_values_from_union_type_literal(union_type),
      TSType::TSTypeReference(type_reference) => {
        if let TSTypeName::IdentifierReference(identifier) = &type_reference.type_name {
          self.program.body.iter().find_map(|stmt| {
            match stmt {
              Statement::TSTypeAliasDeclaration(type_alias) if type_alias.id.name == identifier.name => {
                if let TSType::TSUnionType(union_type) = &type_alias.type_annotation {
                  get_string_values_from_union_type_literal(union_type)
                } else {
                  None
                }
              },
              Statement::ImportDeclaration(import_decl)
                if import_decl.specifiers.as_ref().is_some_and(|specifiers| {
                  specifiers.iter().any(|specifier| {
                    match &specifier {
                      ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        specifier.local.name.eq(&identifier.name)
                      },
                      _ => false,
                    }
                  })
                }) =>
              {
                log::warn!(
                  "{} Value of identifier {} is an import declaration, which is not supported",
                  "[Find_identifier_value_as_vec_string]".red().bold(),
                  identifier.name.cyan()
                );
                None
              },
              _ => None,
            }
          })
        } else {
          log::warn!(
            "{} Unsupported type reference: {type_reference:?}",
            "[Find_identifier_value_as_vec_string]".red().bold()
          );
          None::<Vec<String>>
        }
      },
      _ => {
        log::warn!(
          "{} Unsupported type annotation: {type_annotation:?}",
          "[Find_identifier_value_as_vec_string]".red().bold()
        );
        None::<Vec<String>>
      },
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

  pub(super) fn parse_option_and_default_value(
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

  pub(super) fn parse_value_from_statement(
    &self,
    stmt: &Statement<'_>,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
  ) -> Option<Value> {
    if let Some(decl) = stmt.as_declaration() {
      self.parse_value_from_declaration(identifier, decl)
    } else if let Some(module) = stmt.as_module_declaration() {
      match module {
        ModuleDeclaration::ExportNamedDeclaration(decl) => {
          decl.declaration.as_ref().and_then(|declaration| self.parse_value_from_declaration(identifier, declaration))
        },
        _ => {
          #[cfg(test)]
          todo!("Handle other module declarations: {module:?}");

          None
        },
      }
    } else {
      #[cfg(test)]
      warn!("{} Unsupported statement: {stmt:?}", "[Parse_value_from_statement]".red().bold());
      None
    }
  }

  pub(super) fn parse_value_from_declaration(
    &self,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
    decl: &Declaration<'_>,
  ) -> Option<Value> {
    match decl {
      Declaration::VariableDeclaration(var) => {
        var
          .declarations
          .iter()
          .find(|e| e.id.get_identifier_name().is_some_and(|idx| identifier.name.eq(&idx)))
          .and_then(|item| {
            trace!("Parsing item: {:?}", item);
            item
              .init
              .as_ref()
              .and_then(|init| {
                trace!("Parsing item value: {:?}", init);
                self.parse_expression_to_serde_value(init)
              })
              .or_else(|| {
                trace!("Parsing type annotation for item: {:?}", item);
                item.id.type_annotation.as_ref().and_then(|type_annotation| {
                  self
                    .parse_type_annotation_as_vec_str(type_annotation)
                    .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
                })
              })
          })
      },
      Declaration::FunctionDeclaration(func)
        if func.params.iter_bindings().any(|param| find_type_of_identifier(identifier, param).is_some()) =>
      {
        func.params.iter_bindings().find_map(|param| find_type_of_identifier(identifier, param)).and_then(
          |type_annotation| {
            trace!("Type annotation: {:?}", type_annotation);
            self
              .parse_type_annotation_as_vec_str(type_annotation)
              .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
          },
        )
      },
      Declaration::FunctionDeclaration(func) if func.body.is_some() => {
        func.body.as_ref().unwrap().statements.iter().find_map(|stmt| self.parse_value_from_statement(stmt, identifier))
      },
      _exported => {
        #[cfg(test)]
        warn!("{:?} is not supported for now", _exported);
        None
      },
    }
  }
}

fn find_type_of_identifier<'a>(
  identifier: &oxc_allocator::Box<'a, IdentifierReference<'a>>,
  param: &'a BindingPattern<'a>,
) -> Option<&'a oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
  match &param.kind {
    BindingPatternKind::BindingIdentifier(idx) if idx.name.eq(&identifier.name) => param.type_annotation.as_ref(),
    BindingPatternKind::ObjectPattern(obj) => {
      obj.properties.iter().find(|prop| prop.key.name().is_some_and(|name| name.eq(&identifier.name))).and_then(
        |prop| {
          prop.value.type_annotation.as_ref().or(param.type_annotation.as_ref().and_then(|type_annotation| {
            match &type_annotation.type_annotation {
              TSType::TSTypeLiteral(type_literal) => {
                type_literal.members.iter().find_map(|member| {
                  match member {
                    TSSignature::TSPropertySignature(signature)
                      if signature.key.name().is_some_and(|name| name.eq(&identifier.name)) =>
                    {
                      signature.type_annotation.as_ref()
                    },
                    _ => None,
                  }
                })
              },
              _ => None,
            }
          }))
        },
      )
    },
    _ => None,
  }
}

fn get_string_values_from_union_type_literal(
  union_type: &oxc_allocator::Box<'_, TSUnionType<'_>>,
) -> Option<Vec<String>> {
  let found = union_type
    .types
    .iter()
    .filter_map(|t| {
      if let TSType::TSLiteralType(literal) = t {
        if let TSLiteral::StringLiteral(value) = &literal.literal {
          return Some(value.value.to_string());
        }
      }
      None::<String>
    })
    .collect::<Vec<_>>();

  if found.is_empty() {
    None::<Vec<String>>
  } else {
    trace!("Found union type: {found:?}");
    Some(found)
  }
}
