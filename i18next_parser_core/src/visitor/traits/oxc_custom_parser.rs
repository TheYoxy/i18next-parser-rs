
use color_eyre::owo_colors::OwoColorize;
use log::{debug, trace, warn};
use oxc_ast::ast::{
  BindingPattern,
  BindingPatternKind,
  Declaration,
  Expression,
  IdentifierName,
  IdentifierReference,
  ImportDeclarationSpecifier,
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
  helper::SerdeHelper,
  visitor::traits::{oxc_program::OxcProgram, print_error_location::PrintErrorLocation},
};

pub trait OxcCustomParser: OxcProgram + PrintErrorLocation {
  /// Find the value of an identifier in a statement
  fn find_value_for_identifier(
    &self,
    stmt: &Statement<'_>,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
  ) -> Option<Value> {
    if let Some(decl) = stmt.as_declaration() {
      self.parse_value_for_identifier_from_declaration(identifier, decl)
    } else {
      #[cfg(test)]
      warn!("Unsupported statement: {stmt:?}");
      None
    }
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
  fn find_identifier_value_as_string_from_identifier_name(
    &self,
    identifier: &oxc_allocator::Box<IdentifierName>,
  ) -> Option<String> {
    debug!("Looking for identifier value: {}", identifier.name);
    let arr = self.program().body.iter().find_map(|stmt| {
      if let Statement::VariableDeclaration(var) = stmt {
        var
          .declarations
          .iter()
          .find(|v| v.id.get_identifier_name().is_some_and(|name| name.eq(&identifier.name)))
          .and_then(|item| item.init.as_ref())
          .and_then(|init| self.parse_expression_to_serde_value(init))
      } else {
        None
      }
    });

    if arr.is_none() {
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot find value of {name} {identifier:?}",
        "[Find_identifier_value]".red().bold(),
        name = identifier.name.cyan()
      );
      self.print_error_location(&identifier.span());
    }

    arr.value_to_string()
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
  fn find_identifier_value_as_serde(&self, identifier: &oxc_allocator::Box<IdentifierReference>) -> Option<Value> {
    debug!("Looking for identifier value: {}", identifier.name.cyan());
    let arr = self.program().body.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier));

    if arr.is_none() {
      warn!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.name.cyan()
      );
      self.print_error_location(&identifier.span());
    }

    arr
  }

  fn find_identifier_value_as_vec_string(
    &self,
    identifier: &oxc_allocator::Box<IdentifierReference>,
  ) -> Option<Vec<String>> {
    self.find_identifier_value_as_serde(identifier).value_to_string_vec()
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
    self.find_identifier_value_as_serde(identifier).value_to_string()
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
  fn parse_expression_to_serde_value(&self, expr: &Expression<'_>) -> Option<Value> {
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
      Expression::CallExpression(call) => self.parse_expression_to_serde_value(&call.callee),
      _ => {
        self.print_error_location(&expr.span());
        debug!("{} Unsupported expression: {expr:?}", "[Parse_expression]".red().bold());
        None
      },
    }
  }

  fn parse_value_for_identifier_from_declaration(
    &self,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
    decl: &Declaration<'_>,
  ) -> Option<Value> {
    #[cfg(test)]
    debug!("Parsing declaration: {:?}", decl.bright_black().italic());

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
      Declaration::FunctionDeclaration(func)
        if func.id.as_ref().is_some_and(|idx| identifier.name.eq(&idx.name)) && func.return_type.is_some() =>
      {
        #[cfg(test)]
        debug!("Parsing function return type: {:?}", func.bright_black().italic());
        self
          .parse_type_annotation_as_vec_str(func.return_type.as_ref().unwrap())
          .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
      },
      Declaration::FunctionDeclaration(func) if func.body.is_some() => {
        #[cfg(test)]
        debug!("Parsing function declaration body: {:?}", func.bright_black().italic());
        func.body.as_ref().unwrap().statements.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier))
      },
      _exported => {
        #[cfg(test)]
        warn!("{:?} is not supported for now", _exported);
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
    self.parse_expression_to_serde_value(expr).value_to_string()
  }

  fn parse_type_annotation_as_vec_str(
    &self,
    type_annotation: &oxc_allocator::Box<'_, TSTypeAnnotation<'_>>,
  ) -> Option<Vec<String>> {
    match &type_annotation.type_annotation {
      TSType::TSUnionType(union_type) => get_string_values_from_union_type_literal(union_type),
      TSType::TSTypeReference(type_reference) => {
        if let TSTypeName::IdentifierReference(identifier) = &type_reference.type_name {
          self.program().body.iter().find_map(|stmt| {
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
    trace!("Found union type: {found:?}", found = found.yellow());
    Some(found)
  }
}
