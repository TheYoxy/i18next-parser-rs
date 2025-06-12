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
      log::trace!("Parsing declaration {:?} for identifier: {}", decl.bright_black().italic(), identifier.cyan());
      self.parse_value_for_identifier_from_declaration(identifier, decl)
    } else if let Some(module) = stmt.as_module_declaration() {
      #[cfg(test)]
      log::trace!("Parsing module {:?} for identifier: {}", module.bright_black().italic(), identifier.cyan());
      self.parse_value_from_module_declaration(identifier, module)
    } else {
      #[cfg(test)]
      warn!("Unsupported statement: {stmt:?}");
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
    debug!("Looking for identifier value: {}", identifier.cyan());
    let arr = self.program().body.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier));

    if arr.is_none() {
      #[cfg(test)]
      warn!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
      #[cfg(not(test))]
      debug!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
      // self.print_error_location(&identifier.span());
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
        self.print_error_location(&expr.span());
        debug!("{} Unsupported expression: {expr:?}", "[Parse_expression]".red().bold());
        None
      },
    }
  }

  fn parse_value_for_identifier_from_declaration(&self, identifier: &str, decl: &Declaration<'_>) -> Option<Value> {
    #[cfg(test)]
    debug!("Parsing declaration: {:?}", decl.bright_black().italic());

    let parse_type_annotation = |type_annotation: &oxc_allocator::Box<'_, TSTypeAnnotation<'_>>| {
      #[cfg(test)]
      trace!("Type annotation: {:?}", type_annotation);
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
            trace!("Parsing item: {:?}", item.bright_black().italic());
            item
              .init
              .as_ref()
              .and_then(|init| {
                #[cfg(test)]
                trace!("Parsing item value: {:?}", init.bright_black().italic());
                self.parse_expression_to_serde_value(init)
              })
              .or_else(|| {
                #[cfg(test)]
                trace!("Parsing type annotation for item: {:?}", item.bright_black().italic());
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
          .and_then(parse_type_annotation)
          .or_else(|| {
            #[cfg(test)]
            trace!("Parsing function return type: {:?}", func.bright_black().italic());
            func.return_type.as_ref().and_then(parse_type_annotation)
          })
          .or_else(|| {
            #[cfg(test)]
            trace!("Parsing function declaration body: {:?}", func.bright_black().italic());
            func
              .body
              .as_ref()
              .and_then(|body| body.statements.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier)))
          })
      },
      Declaration::TSTypeAliasDeclaration(type_alias) if type_alias.id.name.eq(identifier) => {
        #[cfg(test)]
        trace!("Parsing type alias: {:?}", type_alias.bright_black().italic());
        self
          .parse_type_annotation_as_vec_str(&type_alias.type_annotation)
          .map(|values| Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()))
      },
      _exported => {
        #[cfg(test)]
        warn!("{:?} is not supported for now", _exported.bright_black().italic());
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
    #[cfg(test)]
    trace!("Parsing type annotation: {:?}", ts_type.bright_black().italic());

    match &ts_type {
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
                              "[parse_type_annotation_as_vec_str]".red().bold(),
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
                      "[parse_type_annotation_as_vec_str]".red().bold(),
                      identifier.name.cyan()
                    );
                    self.print_error_location(&identifier.span());
                  }
                }

                result.value_to_string_vec()
              },
              _ => None,
            }
          })
        } else {
          #[cfg(test)]
          log::warn!(
            "{} Unsupported type reference: {type_reference:?}",
            "[parse_type_annotation_as_vec_str]".red().bold(),
            type_reference = type_reference.type_name
          );
          None::<Vec<String>>
        }
      },
      _ => {
        log::warn!("{} Unsupported type annotation: {ts_type:?}", "[parse_type_annotation_as_vec_str]".red().bold());
        None::<Vec<String>>
      },
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
                  _ => None,
                }
              })
            },
            _ => None,
          }
        }))
      })
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
