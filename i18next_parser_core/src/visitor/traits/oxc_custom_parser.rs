use color_eyre::owo_colors::OwoColorize;
use log::{debug, error, trace, warn};
use oxc_ast::ast::{
  BindingPattern,
  BindingPatternKind,
  Declaration,
  Expression,
  ImportDeclarationSpecifier,
  ModuleDeclaration,
  ModuleExportName,
  ObjectPropertyKind,
  Statement,
  TSLiteral,
  TSSignature,
  TSType,
  TSTypeAnnotation,
  TSTypeName,
  TSUnionType,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

use crate::{
  helper::SerdeHelper,
  visitor::traits::{oxc_program::OxcProgram, print_error_location::PrintErrorLocation},
};

pub trait OxcCustomParser: OxcProgram + PrintErrorLocation {
  /// Find the value of an identifier in a statement
  fn find_value_for_identifier(&self, stmt: &Statement<'_>, identifier: &str) -> Option<Value> {
    if let Some(decl) = stmt.as_declaration() {
      #[cfg(test)]
      log::trace!(
        "Looking for identifier: {identifier} in declaration: {declaration:?}",
        declaration = decl.bright_black().italic(),
        identifier = identifier.cyan()
      );
      self.parse_value_for_identifier_from_declaration(identifier, decl)
    } else if let Some(module) = stmt.as_module_declaration() {
      #[cfg(test)]
      log::trace!("Parsing module {:?} for identifier: {}", module.bright_black().italic(), identifier.cyan());
      self.parse_value_from_module_declaration(identifier, module)
    } else {
      #[cfg(test)]
      warn!("Unsupported statement: {stmt:?}", stmt = stmt.bright_black().italic());
      None
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
  fn find_identifier_value_as_serde(&self, identifier: &str) -> Option<Value> {
    trace!("Looking for identifier value: {}", identifier.cyan());
    let arr = self.program().body.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier));

    if arr.is_none() {
      #[cfg(test)]
      warn!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
      #[cfg(not(test))]
      trace!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
    }

    arr
  }

  fn find_identifier_value_as_vec_string(&self, identifier: &str) -> Option<Vec<String>> {
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
  fn find_identifier_value_as_string(&self, identifier: &str) -> Option<String> {
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
      Expression::Identifier(identifier) => self.find_identifier_value_as_serde(&identifier.name),
      Expression::TSSatisfiesExpression(expr) => self.parse_expression_to_serde_value(&expr.expression),
      Expression::TSAsExpression(expression) => self.parse_expression_to_serde_value(&expression.expression),
      Expression::CallExpression(call) => self.parse_expression_to_serde_value(&call.callee),
      _ => {
        #[cfg(test)]
        {
          self.print_error_location(&expr.span());
          warn!("{} Unsupported expression: {expr:?}", "[Parse_expression]".red().bold());
        }
        None
      },
    }
  }

  fn parse_value_for_identifier_from_declaration(&self, identifier: &str, decl: &Declaration<'_>) -> Option<Value> {
    let parse_type_annotation = |type_annotation: &oxc_allocator::Box<'_, TSTypeAnnotation<'_>>| {
      #[cfg(test)]
      trace!("Type annotation: {:?}", type_annotation.bright_black());
      self
        .parse_type_annotation_as_vec_str(&type_annotation.type_annotation)
        .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
    };

    match decl {
      Declaration::VariableDeclaration(var) => {
        var
          .declarations
          .iter()
          .find(|e| e.id.get_identifier_name().is_some_and(|idx| idx.eq(identifier)))
          .and_then(|item| {
            #[cfg(test)]
            trace!("{} Parsing item: {:?}", "[VariableDeclaration]".blue(), item.bright_black().italic());
            item
              .init
              .as_ref()
              .and_then(|init| {
                #[cfg(test)]
                trace!("{} Parsing item value: {:?}", "[VariableDeclaration]".blue(), init.bright_black().italic());
                self.parse_expression_to_serde_value(init)
              })
              .or_else(|| {
                #[cfg(test)]
                trace!(
                  "{} Parsing type annotation for item: {:?}",
                  "[VariableDeclaration]".blue(),
                  item.bright_black().italic()
                );
                item.id.type_annotation.as_ref().and_then(parse_type_annotation)
              })
          })
          .or_else(|| {
            var.declarations.iter().find_map(|e| {
              e.init.as_ref().and_then(|init| {
                if let Expression::ArrowFunctionExpression(arrow_function) = init {
                  arrow_function
                    .params
                    .iter_bindings()
                    .find_map(|param| find_type_of_identifier(identifier, param))
                    .and_then(parse_type_annotation)
                } else {
                  None
                }
              })
            })
          })
      },
      Declaration::FunctionDeclaration(func) => {
        func
          .params
          .iter_bindings()
          .find_map(|param| find_type_of_identifier(identifier, param))
          .and_then(|type_annotation| {
            trace!("{} Parsing function arguments: {:?}", "[FunctionDeclaration]".blue(), func.bright_black().italic());
            parse_type_annotation(type_annotation)
          })
          .or_else(|| {
            #[cfg(test)]
            trace!(
              "{} Parsing function return type: {:?}",
              "[FunctionDeclaration]".blue(),
              func.bright_black().italic()
            );
            func.return_type.as_ref().and_then(parse_type_annotation)
          })
          .or_else(|| {
            #[cfg(test)]
            trace!(
              "{} Parsing function declaration body: {:?}",
              "[FunctionDeclaration]".blue(),
              func.bright_black().italic()
            );
            func
              .body
              .as_ref()
              .and_then(|body| body.statements.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier)))
          })
      },
      Declaration::TSTypeAliasDeclaration(type_alias) if type_alias.id.name.eq(identifier) => {
        #[cfg(test)]
        trace!("{} Parsing type alias: {:?}", "[TSTypeAliasDeclaration]".blue(), type_alias.bright_black().italic());
        self
          .parse_type_annotation_as_vec_str(&type_alias.type_annotation)
          .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
      },
      _exported => {
        #[cfg(test)]
        warn!(
          "Declaration of {identifier} is not supported {exported:?}",
          identifier = identifier.cyan().italic(),
          exported = _exported.bright_black().italic()
        );
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

  fn parse_type_annotation_as_vec_str(&self, ts_type: &TSType<'_>) -> Option<Vec<String>> {
    debug!("Parsing type annotation: {:?}", ts_type.bright_black().italic());

    match &ts_type {
      TSType::TSUnionType(union_type) => get_string_values_from_union_type_literal(union_type),
      TSType::TSTypeQuery(type_query) => {
        let val =
          type_query.expr_name.as_ts_type_name().and_then(|type_name| self.parse_type_name_as_vec_str(type_name));
        val
      },
      TSType::TSParenthesizedType(parenthesized) => {
        self.parse_type_annotation_as_vec_str(&parenthesized.type_annotation)
      },
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_keyword() => {
        self.parse_type_annotation_as_vec_str(&indexed.object_type)
      },
      TSType::TSTypeReference(type_reference) => self.parse_type_name_as_vec_str(&type_reference.type_name),
      _ => {
        log::warn!("{} Unsupported type annotation: {ts_type:?}", "[parse_type_annotation_as_vec_str]".red().bold());
        None
      },
    }
  }

  fn parse_type_name_as_vec_str(&self, type_name: &oxc_ast::ast::TSTypeName<'_>) -> Option<Vec<String>> {
    if let TSTypeName::IdentifierReference(identifier) = &type_name {
      debug!("Looking for type reference {} {:?}", identifier.name.cyan(), type_name.bright_black());
      let val = self.program().body.iter().find_map(|stmt| {
        stmt
          .as_declaration()
          .and_then(|decl| {
              if let Declaration::VariableDeclaration(var) = decl {
                  if var.declarations.iter().any(|decl| decl.id.get_identifier_name().is_some_and(|name| name.eq(&identifier.name))) {
                      self.parse_value_for_identifier_from_declaration(&identifier.name, decl).value_to_string_vec()
                  } else {
                      None
                  }

              } else if decl.is_type() {
              self.parse_value_for_identifier_from_declaration(&identifier.name, decl).value_to_string_vec()
            } else if decl.id().is_some_and(|id| id.name == identifier.name) {
              self.parse_value_for_identifier_from_declaration(&identifier.name, decl).value_to_string_vec()
            } else {
              #[cfg(test)]
              log::trace!(
                "Skipping declaration {decl:?} [Type:{is_type}] for identifier {identifier:?} as it does not match {raw_decl:?}",
                decl = decl.id().map(|id| id.name).blue(),
                is_type = decl.is_type(),
                identifier = identifier.name.cyan(),
                raw_decl = decl.bright_black()
              );
              None
            }
          })
          .or_else(|| {
            match stmt {
              Statement::TSTypeAliasDeclaration(type_alias) if type_alias.id.name == identifier.name => {
                match &type_alias.type_annotation {
                  TSType::TSTypeReference(type_reference) if type_reference.type_name.eq(type_name) => {
                    log::debug!(
                      "Skipping type {type_name} to avoid infinite loop",
                      type_name = type_name.bright_black()
                    );
                    None
                  },
                  _ => self.parse_type_annotation_as_vec_str(&type_alias.type_annotation),
                }
              },
              Statement::ImportDeclaration(import_decl) => {
                let result = import_decl.specifiers.as_ref().and_then(|specifiers| {
                  specifiers.iter().find_map(|specifier| {
                    if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
                      if specifier.local.name.eq(&identifier.name) {
                        match &specifier.imported {
                          ModuleExportName::IdentifierReference(identifier_reference) => {
                            self.find_value_identifier_and_declaration(
                              &import_decl.source.value,
                              &identifier_reference.name,
                            )
                          },
                          ModuleExportName::IdentifierName(name) => {
                            trace!("Identifier name: {name:?}");
                            self.find_value_identifier_and_declaration(&import_decl.source.value, &name.name)
                          },
                          _ => {
                            #[cfg(test)]
                            warn!(
                              "{} Unsupported import specifier: {:?}",
                              "[parse_type_name_as_vec_str]".red().bold(),
                              specifier.imported
                            );
                            None
                          },
                        }
                      } else {
                        None
                      }
                    } else {
                      None
                    }
                  })
                });

                #[cfg(debug_assertions)]
                {
                  if result.is_none() {
                    log::warn!(
                      "{} Value of identifier {} is an import declaration, which is not supported",
                      "[parse_type_name_as_vec_str]".red().bold(),
                      identifier.name.cyan()
                    );
                    self.print_error_location(&identifier.span());
                  }
                }

                result.value_to_string_vec()
              },
              _statement => {
                #[cfg(test)]
                log::warn!(
                  "{} Unsupported statement {:?}",
                  "[parse_type_name_as_vec_str]".red().bold(),
                  _statement.bright_black()
                );
                None
              },
            }
          })
      });

      #[cfg(debug_assertions)]
      {
        if val.is_none() {
          log::warn!(
            "{} Cannot find value for identifier: {identifier:?}",
            "parse_type_name_as_vec_str".red().bold(),
            identifier = identifier.name.cyan()
          );
        }
      }

      val
    } else {
      #[cfg(test)]
      log::warn!(
        "{} Unsupported type reference: {type_name:?}",
        "parse_type_name_as_vec_str".red().bold(),
        type_name = type_name.bright_black()
      );
      None::<Vec<String>>
    }
  }

  /// Parse a value from a module declaration
  fn parse_value_from_module_declaration(&self, identifier: &str, module: &ModuleDeclaration<'_>) -> Option<Value> {
    match module {
      ModuleDeclaration::ExportNamedDeclaration(decl) => {
        decl.declaration.as_ref().and_then(|declaration| {
          #[cfg(test)]
          log::trace!(
            "Parsing value from module declaration: {declaration:?} for {identifier:?}",
            declaration = declaration.bright_black().italic(),
            identifier = identifier.cyan()
          );
          let result = self.parse_value_for_identifier_from_declaration(identifier, declaration);
          #[cfg(test)]
          log::trace!("Result: {result:?}", result = result.yellow());
          result
        })
      },
      ModuleDeclaration::ImportDeclaration(import)
        if import.specifiers.as_ref().is_some_and(|specifiers| specifiers.iter().any(|s| s.name().eq(&identifier))) =>
      {
        self.find_value_identifier_and_declaration(&import.source.value, identifier)
      },
      _ => {
        #[cfg(test)]
        warn!("Handle other module declarations: {module:?}");

        None
      },
    }
  }

  fn find_value_identifier_and_declaration(&self, path_to_resolve: &str, identifier: &str) -> Option<Value> {
    let path =
      (if self.file_path().is_dir() { Some(self.file_path().as_path()) } else { self.file_path().parent() }).unwrap();

    let resolved = self.resolver().resolve(
      path,
      if path_to_resolve == "." {
        warn!("Resolving path {path_to_resolve} as index");
        "index"
      } else {
        path_to_resolve
      },
    );
    match resolved {
      Ok(resolved) => {
        let path = resolved.path();
        let source_type = SourceType::from_path(path).unwrap();
        let source_text = std::fs::read_to_string(path).unwrap();
        let parser = Parser::new(self.allocator(), source_text.as_str(), source_type);
        let result = parser.parse();
        if result.panicked {
          warn!(
            "{} Failed to parse file {}: {:?}",
            "[Parse_i18next_option]".red().bold(),
            path.display().yellow(),
            result.errors
          );
          return None;
        }

        result.program.body.iter().find_map(|stmt| {
          #[cfg(test)]
          log::trace!(
            "Finding value for identifier from {} {identifier:?} in statement: {stmt:?}",
            path.display().yellow(),
            identifier = identifier.cyan(),
            stmt = stmt.bright_black().italic()
          );
          if let Some(module) = stmt.as_module_declaration() {
            let r = self.parse_value_from_module_declaration(identifier, module);
            #[cfg(test)]
            log::trace!("Found value: {r:?}", r = r.yellow());
            r
          } else {
            None
          }
        })
      },
      Err(err) => {
        error!(
          "{} Failed to resolve import to {}: {}",
          "[Parse_value_from_statement]".on_red().bold(),
          path_to_resolve.yellow(),
          err.red()
        );
        None
      },
    }
  }
}

fn find_type_of_identifier<'a>(
  identifier: &str,
  param: &'a BindingPattern<'a>,
) -> Option<&'a oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
  match &param.kind {
    BindingPatternKind::BindingIdentifier(idx) if idx.name.eq(&identifier) => param.type_annotation.as_ref(),
    BindingPatternKind::ObjectPattern(obj) => {
      obj.properties.iter().find(|prop| prop.key.name().is_some_and(|name| name.eq(&identifier))).and_then(|prop| {
        prop.value.type_annotation.as_ref().or(param.type_annotation.as_ref().and_then(|type_annotation| {
          match &type_annotation.type_annotation {
            TSType::TSTypeLiteral(type_literal) => {
              type_literal.members.iter().find_map(|member| {
                match member {
                  TSSignature::TSPropertySignature(signature)
                    if signature.key.name().is_some_and(|name| name.eq(&identifier)) =>
                  {
                    signature.type_annotation.as_ref()
                  },
                  _ => {
                    #[cfg(test)]
                    warn!(
                      "{} Unsupported type annotation in object pattern: {type_annotation:?}",
                      "[find_type_of_identifier]".red().bold(),
                      type_annotation = type_annotation.type_annotation
                    );
                    None
                  },
                }
              })
            },
            _ => {
              #[cfg(test)]
              warn!(
                "{} Unsupported type annotation in object pattern: {type_annotation:?}",
                "[find_type_of_identifier]".red().bold(),
                type_annotation = type_annotation.type_annotation
              );
              None
            },
          }
        }))
      })
    },
    _ => {
      #[cfg(test)]
      warn!(
        "{} Unsupported binding pattern for identifier {identifier}: {param:?}",
        "[find_type_of_identifier]".red().bold(),
        identifier = identifier.cyan(),
        param = param.bright_black().italic()
      );
      None
    },
  }
}

fn get_string_values_from_union_type_literal(
  union_type: &oxc_allocator::Box<'_, TSUnionType<'_>>,
) -> Option<Vec<String>> {
  trace!("Parsing values from union {:?}", union_type.bright_black());
  let found = union_type
    .types
    .iter()
    .filter_map(|t| {
      if let TSType::TSLiteralType(literal) = t {
        if let TSLiteral::StringLiteral(value) = &literal.literal {
          trace!("Found value: {}", value.value.cyan());
          return Some(value.value.to_string());
        }
      }
      None::<String>
    })
    .collect::<Vec<_>>();

  if found.is_empty() {
    #[cfg(test)]
    warn!("No union type found from {:?}", union_type.bright_black().italic());
    None::<Vec<String>>
  } else {
    trace!("Found union type: {found:?}", found = found.yellow());
    Some(found)
  }
}

trait TypeNameEq {
  fn eq(&self, other: &Self) -> bool;
}
impl TypeNameEq for oxc_ast::ast::TSTypeName<'_> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (TSTypeName::IdentifierReference(a), TSTypeName::IdentifierReference(b)) => {
        trace!("{:?} == {:?}", a.span, b.span);
        a.span == b.span
      },
      (TSTypeName::QualifiedName(a), TSTypeName::QualifiedName(b)) => a.span == b.span,
      _ => false,
    }
  }
}
