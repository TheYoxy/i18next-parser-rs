use std::panic;

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
use oxc_span::SourceType;
use serde_json::Value;

use crate::{
  helper::SerdeHelper,
  visitor::{
    parser::ModuleParser,
    traits::{oxc_program::OxcProgram, print_error_location::PrintErrorLocation},
  },
};

pub trait OxcCustomParser: OxcProgram + PrintErrorLocation {
  /// Find the value of an identifier in a statement
  fn find_value_for_identifier(&self, stmt: &Statement<'_>, identifier: &str) -> Option<Value> {
    if let Some(decl) = stmt.as_declaration() {
      self.parse_value_for_identifier_from_declaration(identifier, decl)
    } else if let Some(module) = stmt.as_module_declaration() {
      self.parse_value_from_module_declaration(identifier, module)
    } else {
      #[cfg(debug_assertions)]
      {
        warn!(
          "{} Unsupported statement: {stmt:?}",
          "[find_value_for_identifier]".red().bold(),
          stmt = stmt.bright_black().italic()
        );
      }
      #[cfg(not(debug_assertions))]
      {
        debug!(
          "{} Unsupported statement: {}",
          "[find_value_for_identifier]".red().bold(),
          std::any::type_name_of_val(&stmt)
        );
      }
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
    trace!("Looking for identifier value: {} in {}", identifier.cyan(), self.file_path().display().yellow());
    let arr = self.program().body.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier));

    if arr.is_none() {
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
      #[cfg(not(debug_assertions))]
      debug!(
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
    #[cfg(test)]
    {
      trace!("Parsing expression to value: {:?}", expr.bright_black().italic());
    }

    let ret = match expr {
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
                trace!("Parsing array expession");
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
                trace!("Parsing object expression");
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
      Expression::TSSatisfiesExpression(expr) => {
        self.parse_expression_to_serde_value(&expr.expression).or(self.parse_ts_type(&expr.type_annotation))
      },
      Expression::TSAsExpression(expression) => {
        self.parse_expression_to_serde_value(&expression.expression).or(self.parse_ts_type(&expression.type_annotation))
      },
      Expression::CallExpression(call) => self.parse_expression_to_serde_value(&call.callee),
      _ => {
        #[cfg(debug_assertions)]
        {
          use oxc_span::GetSpan;
          self.print_error_location(&expr.span());
          #[cfg(test)]
          warn!(
            "{} Unsupported expression: {expr:?}",
            "[Parse_expression]".red().bold(),
            expr = expr.bright_black().italic()
          );
          #[cfg(not(test))]
          warn!(
            "{} Unsupported expression: {expr}",
            "[Parse_expression]".red().bold(),
            expr = std::any::type_name_of_val(&expr)
          );
        }
        None
      },
    };

    if let Some(ret) = &ret {
      trace!("Found value: {ret}");
    } else {
      trace!("{} found for expression: {:?}", "No value".red().bold(), expr.bright_black().italic());
    }

    ret
  }

  fn parse_value_for_identifier_from_declaration(&self, identifier: &str, decl: &Declaration<'_>) -> Option<Value> {
    let parse_type_annotation = |type_annotation: &oxc_allocator::Box<'_, TSTypeAnnotation<'_>>| {
      #[cfg(test)]
      trace!("Type annotation: {:?}", type_annotation.bright_black());

      self.parse_ts_type(&type_annotation.type_annotation)
    };

    match decl {
      Declaration::VariableDeclaration(var) => {
        var
          .declarations
          .iter()
          .find(|e| e.id.get_identifier_name().is_some_and(|idx| idx.eq(identifier)))
          .and_then(|item| {
            #[cfg(test)]
            debug!("{} Parsing item: {:?}", "[VariableDeclaration]".blue(), item.bright_black().italic());
            #[cfg(not(test))]
            trace!("{} Parsing item: {}", "[VariableDeclaration]".blue(), std::any::type_name_of_val(item));
            item
              .init
              .as_ref()
              .and_then(|init| {
                #[cfg(test)]
                debug!("{} Parsing item value: {:?}", "[VariableDeclaration]".blue(), init.bright_black().italic());
                #[cfg(not(test))]
                trace!("{} Parsing item value: {}", "[VariableDeclaration]".blue(), std::any::type_name_of_val(init));
                self.parse_expression_to_serde_value(init)
              })
              .or_else(|| {
                #[cfg(test)]
                debug!(
                  "{} Parsing type annotation for item: {:?}",
                  "[VariableDeclaration]".blue(),
                  item.bright_black().italic()
                );
                #[cfg(not(test))]
                trace!(
                  "{} Parsing type annotation for item: {}",
                  "[VariableDeclaration]".blue(),
                  std::any::type_name_of_val(item)
                );
                item.id.type_annotation.as_ref().and_then(parse_type_annotation).or_else(|| {
                  item.init.as_ref().and_then(|init| {
                    trace!(
                      "{} Parsing type annotation from init {:?}",
                      "[VariableDeclaration]".blue(),
                      init.bright_black().italic()
                    );
                    self.parse_expression_to_serde_value(init)
                  })
                })
              })
          })
          .or_else(|| {
            var.declarations.iter().find_map(|e| {
              e.init.as_ref().and_then(|init| {
                if let Expression::ArrowFunctionExpression(arrow_function) = init {
                  arrow_function
                    .params
                    .iter_bindings()
                    .find_map(|param| self.find_type_of_identifier(identifier, param))
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
          .find_map(|param| self.find_type_of_identifier(identifier, param))
          .and_then(|type_annotation| {
            #[cfg(test)]
            debug!(
              "{} Parsing function arguments: {:?}",
              "[FunctionDeclaration]".blue(),
              type_annotation.bright_black().italic()
            );
            #[cfg(not(test))]
            trace!(
              "{} Parsing function arguments: {}",
              "[FunctionDeclaration]".blue(),
              std::any::type_name_of_val(type_annotation)
            );
            parse_type_annotation(type_annotation)
          })
          .or_else(|| {
            #[cfg(test)]
            trace!(
              "{} Parsing function return type: {:?}",
              "[FunctionDeclaration]".blue(),
              func.id.bright_black().italic()
            );
            #[cfg(not(test))]
            trace!(
              "{} Parsing function return type: {}",
              "[FunctionDeclaration]".blue(),
              std::any::type_name_of_val(&func.id)
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
            #[cfg(not(test))]
            trace!(
              "{} Parsing function declaration body: {}",
              "[FunctionDeclaration]".blue(),
              std::any::type_name_of_val(func)
            );
            func
              .body
              .as_ref()
              .and_then(|body| body.statements.iter().find_map(|stmt| self.find_value_for_identifier(stmt, identifier)))
          })
      },
      Declaration::TSTypeAliasDeclaration(type_alias) if type_alias.id.name.eq(identifier) => {
        #[cfg(test)]
        trace!(
          "{} Parsing type alias for {}: {:?}",
          "[TSTypeAliasDeclaration]".blue(),
          identifier.cyan(),
          type_alias.bright_black().italic()
        );
        #[cfg(not(test))]
        trace!(
          "{} Parsing type alias for {}: {}",
          "[TSTypeAliasDeclaration]".blue(),
          identifier.cyan(),
          std::any::type_name_of_val(type_alias)
        );
        self.parse_ts_type(&type_alias.type_annotation)
      },
      _exported => {
        #[cfg(test)]
        warn!(
          "Declaration of {identifier} is not supported {exported:?}",
          identifier = identifier.cyan().italic(),
          exported = _exported.bright_black().italic()
        );
        #[cfg(not(test))]
        trace!(
          "Declaration of {identifier} is not supported {exported:?}",
          identifier = identifier.cyan().italic(),
          exported = std::any::type_name_of_val(&_exported)
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

  fn parse_ts_type(&self, ts_type: &TSType<'_>) -> Option<Value> {
    trace!("{} Parsing type annotation: {:?}", "[parse_ts_type]".blue(), ts_type.bright_black().italic());

    match &ts_type {
      TSType::TSLiteralType(literal) => get_value_from_literal_type(literal),
      TSType::TSUnionType(union_type) => {
        get_string_values_from_union_type_literal(union_type)
          .map(|v| Value::Array(v.iter().map(|val| Value::String(val.clone())).collect()))
      },
      TSType::TSTypeQuery(type_query) => {
        trace!("Type query: {:?}", type_query.bright_black().dimmed());
        type_query.expr_name.as_ts_type_name().and_then(|type_name| self.parse_ts_type_name(type_name))
      },
      TSType::TSParenthesizedType(parenthesized) => self.parse_ts_type(&parenthesized.type_annotation),
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_const_type_reference() => None,
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_keyword() => {
        self.parse_ts_type(&indexed.object_type)
      },
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_keyword_or_literal() => {
        self.parse_ts_type(&indexed.index_type).value_to_string().and_then(|key| {
          trace!(
            "{} Looking for {} in {:?}",
            "[parse_ts_type]".blue(),
            key.cyan(),
            indexed.object_type.bright_black().italic()
          );
          let value = self.parse_ts_type(&indexed.object_type);
          if let Some(Value::Object(value)) = value { value.get(&key).cloned() } else { None }
        })
      },
      TSType::TSArrayType(array_type) => self.parse_ts_type(&array_type.element_type),
      TSType::TSTypeReference(type_reference) if type_reference.type_name.is_const() => None,
      TSType::TSTypeReference(type_reference) => {
        match &type_reference.type_name {
          TSTypeName::IdentifierReference(identifier) if identifier.name.eq("Array") => {
            type_reference.type_arguments.as_ref().and_then(|type_args| {
              if type_args.params.len() == 1 {
                self.parse_ts_type(type_args.params.first().expect("only 1 argument"))
              } else {
                self.parse_ts_type_name(&type_reference.type_name)
              }
            })
          },
          _ => self.parse_ts_type_name(&type_reference.type_name),
        }
      },
      TSType::TSTypeLiteral(type_literal) => {
        Some(Value::Object(
          type_literal
            .members
            .iter()
            .filter_map(|signature| {
              match signature {
                TSSignature::TSPropertySignature(property) => {
                  property
                    .type_annotation
                    .as_ref()
                    .and_then(|v| self.parse_ts_type(&v.type_annotation))
                    .map(|value| (property.key.name().unwrap_or_default().to_string(), value))
                },
                _ => None,
              }
            })
            .collect(),
        ))
      },
      _ => {
        #[cfg(test)]
        warn!(
          "{} Unsupported type annotation: {ts_type:?}",
          "[parse_ts_type]".red().bold(),
          ts_type = ts_type.bright_black().italic()
        );
        #[cfg(not(test))]
        debug!(
          "{} Unsupported type annotation: {ts_type}",
          "[parse_ts_type]".red().bold(),
          ts_type = std::any::type_name_of_val(&ts_type)
        );
        None
      },
    }
  }

  fn parse_ts_type_name(&self, type_name: &oxc_ast::ast::TSTypeName<'_>) -> Option<Value> {
    if let TSTypeName::IdentifierReference(identifier) = &type_name {
      trace!(
        "Looking for type reference {} {:?} {:?}",
        identifier.name.cyan(),
        identifier.blue(),
        type_name.bright_black()
      );

      let parse_type_from_declaration = |decl: &Declaration| {
        if let Declaration::VariableDeclaration(var) = decl {
          if var
            .declarations
            .iter()
            .any(|decl| decl.id.get_identifier_name().is_some_and(|name| name.eq(&identifier.name)))
          {
            self.parse_value_for_identifier_from_declaration(&identifier.name, decl)
          } else {
            None
          }
        } else if decl.is_type() || decl.id().is_some_and(|id| id.name == identifier.name) {
          self.parse_value_for_identifier_from_declaration(&identifier.name, decl)
        } else {
          #[cfg(debug_assertions)]
          {
            log::trace!(
              "Skipping declaration {decl:?} [Type:{is_type}] for identifier {identifier:?} as it does not match {raw_decl:?}",
              decl = decl.id().map(|id| id.name).blue(),
              is_type = decl.is_type(),
              identifier = identifier.name.cyan(),
              raw_decl = decl.bright_black()
            );
          }
          None
        }
      };

      let val = self.program().body.iter().find_map(|stmt| {
        stmt.as_declaration().and_then(parse_type_from_declaration).or_else(|| {
          match stmt {
            Statement::TSTypeAliasDeclaration(type_alias) if type_alias.id.name == identifier.name => {
              match &type_alias.type_annotation {
                TSType::TSTypeReference(type_reference) if type_reference.type_name.eq(type_name) => {
                  log::trace!("Skipping type {type_name} to avoid infinite loop", type_name = type_name.bright_black());
                  None
                },
                _ => self.parse_ts_type(&type_alias.type_annotation),
              }
            },
            Statement::ImportDeclaration(import_decl) => {
              let result = import_decl.specifiers.as_ref().and_then(|specifiers| {
                specifiers.iter().find_map(|specifier| {
                  match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier)
                      if specifier.local.name.eq(&identifier.name) =>
                    {
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
                          #[cfg(debug_assertions)]
                          warn!(
                            "{} Unsupported import specifier: {:?}",
                            "[parse_ts_type_name]".red().bold(),
                            specifier.imported.bright_black()
                          );
                          #[cfg(not(debug_assertions))]
                          debug!(
                            "{} Unsupported import specifier: {}",
                            "[parse_ts_type_name]".red().bold(),
                            std::any::type_name_of_val(&specifier.imported)
                          );
                          None
                        },
                      }
                    },
                    _ => None,
                  }
                })
              });

              #[cfg(debug_assertions)]
              {
                use oxc_span::GetSpan;
                if result.is_none() {
                  log::warn!(
                    "{} Value of identifier {} is an import declaration, which is not supported",
                    "[parse_ts_type_name]".red().bold(),
                    identifier.name.cyan()
                  );
                  self.print_error_location(&identifier.span());
                }
              }

              result
            },
            _statement => {
              #[cfg(test)]
              warn!(
                "{} Unsupported statement: {statement:?}",
                "[parse_ts_type_name]".red().bold(),
                statement = _statement.bright_black().italic()
              );
              #[cfg(not(test))]
              debug!(
                "{} Unsupported statement {}",
                "[parse_ts_type_name]".red().bold(),
                std::any::type_name_of_val(_statement)
              );
              None
            },
          }
        })
      });

      if val.is_none() {
        log::info!(
          "{} Cannot find value for identifier: {identifier:?} in {file_name}",
          "[parse_ts_type_name]".red().bold(),
          identifier = identifier.name.cyan(),
          file_name = self.file_path().display().yellow()
        );
      }

      val
    } else {
      #[cfg(test)]
      warn!(
        "{} Unsupported type name: {type_name:?}",
        "[parse_ts_type_name]".red().bold(),
        type_name = type_name.bright_black().italic()
      );
      #[cfg(not(test))]
      debug!(
        "{} Unsupported type name: {}",
        "[parse_ts_type_name]".red().bold(),
        std::any::type_name_of_val(type_name)
      );
      None::<Value>
    }
  }

  /// Parse a value from a module declaration
  fn parse_value_from_module_declaration(&self, identifier: &str, module: &ModuleDeclaration<'_>) -> Option<Value> {
    match module {
      ModuleDeclaration::ExportNamedDeclaration(decl) => {
        decl.declaration.as_ref().and_then(|declaration| {
          #[cfg(test)]
          log::trace!(
            "{} Looking for identifier: {identifier:?} in declaration: {declaration:?}",
            "[ExportNamedDeclaration]".blue(),
            identifier = identifier.cyan(),
            declaration = declaration.bright_black().italic(),
          );
          #[cfg(not(test))]
          log::trace!(
            "{} Looking for identifier: {identifier:?} in declaration: {declaration}",
            "[ExportNamedDeclaration]".blue(),
            identifier = identifier.cyan(),
            declaration = std::any::type_name_of_val(declaration),
          );

          let result = self.parse_value_for_identifier_from_declaration(identifier, declaration);
          if let Some(result) = &result {
            log::trace!("Found: {result}", result = result.yellow());
          } else {
            log::trace!("No value found for {identifier}", identifier = identifier.cyan());
          }
          result
        })
      },
      ModuleDeclaration::ImportDeclaration(import)
        if import.specifiers.as_ref().is_some_and(|specifiers| specifiers.iter().any(|s| s.name().eq(&identifier))) =>
      {
        self.find_value_identifier_and_declaration(&import.source.value, identifier)
      },
      ModuleDeclaration::ImportDeclaration(import) => {
        let specifiers = import
          .specifiers
          .as_ref()
          .map(|specifiers| specifiers.iter().map(|specifier| specifier.name()).collect::<Vec<_>>())
          .unwrap_or(vec![]);
        trace!(
          "Skipping import to {source} since {identifier} is not found in specifiers {specifiers:?}",
          source = import.source.yellow(),
          identifier = identifier.cyan(),
          specifiers = specifiers.bold()
        );
        None
      },
      _module => {
        #[cfg(test)]
        warn!(
          "{} Unsupported module declaration: {module:?}",
          "[parse_value_from_module_declaration]".red().bold(),
          module = _module.bright_black().italic()
        );
        #[cfg(not(test))]
        {
          let _module = match _module {
            ModuleDeclaration::ImportDeclaration(_) => "ImportDeclaration",
            ModuleDeclaration::ExportAllDeclaration(_) => "ExportAllDeclaration",
            ModuleDeclaration::ExportDefaultDeclaration(_) => "ExportDefaultDeclaration",
            ModuleDeclaration::ExportNamedDeclaration(_) => "ExportNamedDeclaration",
            ModuleDeclaration::TSExportAssignment(_) => "TSExportAssignment",
            ModuleDeclaration::TSNamespaceExportDeclaration(_) => "TSNamespaceExportDeclaration",
          };
          debug!(
            "{} Unsupported module declaration: {}",
            "[parse_value_from_module_declaration]".red().bold(),
            _module
          );
        }

        None
      },
    }
  }

  fn find_value_identifier_and_declaration(&self, path_to_resolve: &str, identifier: &str) -> Option<Value> {
    let path =
      (if self.file_path().is_dir() { Some(self.file_path().as_path()) } else { self.file_path().parent() }).unwrap();
    let path_to_resolve = if path_to_resolve == "." {
      warn!("Resolving path {path_to_resolve} as index");
      "index"
    } else {
      path_to_resolve
    };

    trace!(
      "{} resolving {} from {}",
      "[find_value_identifier_and_declaration]".on_green().black(),
      path_to_resolve.yellow(),
      path.display().yellow()
    );
    let resolved = self.resolver().resolve(path, path_to_resolve);
    match resolved {
      Ok(resolved) => {
        trace!(
          "{} resolved to {}",
          "[find_value_identifier_and_declaration]".on_green().black(),
          resolved.path().display().yellow()
        );
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

        let allocator = Default::default();
        let module_parser = ModuleParser::new(&result.program, &allocator, resolved.path().into(), self.working_dir());

        result.program.body.iter().find_map(|stmt| {
          #[cfg(debug_assertions)]
          log::trace!(
            "Finding value for identifier from {} {identifier:?} in statement: {stmt:?}",
            path.display().yellow(),
            identifier = identifier.cyan(),
            stmt = stmt.bright_black().italic()
          );
          #[cfg(not(debug_assertions))]
          trace!(
            "Finding value for identifier from {} {identifier:?} in statement: {stmt}",
            path.display().yellow(),
            identifier = identifier.cyan(),
            stmt = std::any::type_name_of_val(stmt)
          );
          if let Some(module) = stmt.as_module_declaration() {
            log::trace!("Parsing remote declaration for {} {:?}", identifier.cyan(), module.bright_black());
            let r = module_parser.parse_value_from_module_declaration(identifier, module);
            #[cfg(debug_assertions)]
            log::trace!("Found value: {r:?}", r = r.yellow());
            r
          } else {
            None
          }
        })
      },
      Err(err) => {
        if cfg!(test) {
          error!("Resolver options: {:#?}", self.resolver().options().bright_black().italic());

          panic!(
            "{} Failed to resolve import to {} from {}: {}",
            "[find_value_identifier_and_declaration]".on_red().bold(),
            path_to_resolve.yellow(),
            path.display().yellow(),
            err.red()
          );
        }

        None
      },
    }
  }

  fn find_type_of_identifier<'a>(
    &'a self,
    identifier: &'a str,
    param: &'a BindingPattern<'a>,
  ) -> Option<&'a oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
    trace!(
      "{} Looking for {} in {:?}",
      "[find_type_of_identifier]".red().bold(),
      identifier.cyan(),
      param.bright_black().italic()
    );
    match &param.kind {
      BindingPatternKind::BindingIdentifier(idx) if idx.name.eq(&identifier) => param.type_annotation.as_ref(),
      BindingPatternKind::ObjectPattern(obj) => {
        obj.properties.iter().find(|prop| prop.key.name().is_some_and(|name| name.eq(&identifier))).and_then(|prop| {
          prop.value.type_annotation.as_ref().or(param.type_annotation.as_ref().and_then(|type_annotation| {
            self.find_type_of_identifier_from_ts_type(identifier, &type_annotation.type_annotation)
          }))
        })
      },
      _ => {
        #[cfg(debug_assertions)]
        warn!(
          "{} Unsupported binding pattern: {param:?}",
          "[find_type_of_identifier]".red().bold(),
          param = param.bright_black().italic()
        );
        #[cfg(not(debug_assertions))]
        debug!(
          "{} Unsupported binding pattern: {}",
          "[find_type_of_identifier]".red().bold(),
          std::any::type_name_of_val(param)
        );
        None
      },
    }
  }

  fn find_type_of_identifier_from_ts_type<'a>(
    &'a self,
    identifier: &str,
    ts_type: &'a TSType<'a>,
  ) -> Option<&'a oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
    trace!("Looking for {} in {:?}", identifier.cyan(), ts_type.bright_black().italic());

    match ts_type {
      TSType::TSTypeLiteral(type_literal) => {
        type_literal.members.iter().find_map(|member| {
          match member {
            TSSignature::TSPropertySignature(signature)
              if signature.key.name().is_some_and(|name| name.eq(&identifier)) =>
            {
              signature.type_annotation.as_ref()
            },
            _ => {
              #[cfg(debug_assertions)]
              warn!(
                "{} Unsupported type annotation in object pattern: {type_annotation:?}",
                "[find_type_of_identifier_from_ts_type]".red().bold(),
                type_annotation = ts_type.bright_black().italic()
              );
              #[cfg(not(debug_assertions))]
              debug!(
                "{} Unsupported type annotation in object pattern: {}",
                "[find_type_of_identifier_from_ts_type]".red().bold(),
                get_ts_type_name(ts_type)
              );
              None
            },
          }
        })
      },
      TSType::TSTypeReference(type_reference) if type_reference.type_arguments.is_some() => {
        type_reference
          .type_arguments
          .as_ref()
          .and_then(|args| args.params.iter().find_map(|t| self.find_type_of_identifier_from_ts_type(identifier, t)))
      },
      _ => {
        #[cfg(debug_assertions)]
        {
          warn!(
            "{} Unable to find identifier {} from {type_annotation:?}",
            "[find_type_of_identifier_from_ts_type]".red().bold(),
            identifier.cyan(),
            type_annotation = ts_type.bright_black().italic()
          );
          use oxc_span::GetSpan;
          self.print_error_location(&ts_type.span());
        }
        #[cfg(not(debug_assertions))]
        debug!(
          "{} Unable to find identifier {} from {type_annotation}",
          "[find_type_of_identifier_from_ts_type]".red().bold(),
          identifier.cyan(),
          type_annotation = get_ts_type_name(ts_type).bold()
        );

        None
      },
    }
  }
}

fn get_value_from_literal_type(literal: &oxc_allocator::Box<'_, oxc_ast::ast::TSLiteralType<'_>>) -> Option<Value> {
  match &literal.literal {
    TSLiteral::BooleanLiteral(boolean_literal) => Some(Value::Bool(boolean_literal.value)),
    TSLiteral::NumericLiteral(numeric_literal) => {
      serde_json::Number::from_f64(numeric_literal.value).map(Value::Number)
    },
    TSLiteral::BigIntLiteral(_big_int_literal) => None,
    TSLiteral::StringLiteral(string_literal) => Some(Value::String(string_literal.value.to_string())),
    TSLiteral::TemplateLiteral(_template_literal) => None,
    TSLiteral::UnaryExpression(_unary_expression) => None,
  }
}

#[cfg(not(debug_assertions))]
fn get_ts_type_name(ts_type: &TSType) -> &'static str {
  match ts_type {
    TSType::TSAnyKeyword(_) => "TSAnyKeyword",
    TSType::TSBigIntKeyword(_) => "TSBigIntKeyword",
    TSType::TSBooleanKeyword(_) => "TSBooleanKeyword",
    TSType::TSIntrinsicKeyword(_) => "TSIntrinsicKeyword",
    TSType::TSNeverKeyword(_) => "TSNeverKeyword",
    TSType::TSNullKeyword(_) => "TSNullKeyword",
    TSType::TSNumberKeyword(_) => "TSNumberKeyword",
    TSType::TSObjectKeyword(_) => "TSObjectKeyword",
    TSType::TSStringKeyword(_) => "TSStringKeyword",
    TSType::TSSymbolKeyword(_) => "TSSymbolKeyword",
    TSType::TSUndefinedKeyword(_) => "TSUndefinedKeyword",
    TSType::TSUnknownKeyword(_) => "TSUnknownKeyword",
    TSType::TSVoidKeyword(_) => "TSVoidKeyword",
    TSType::TSArrayType(_) => "TSArrayType",
    TSType::TSConditionalType(_) => "TSConditionalType",
    TSType::TSConstructorType(_) => "TSConstructorType",
    TSType::TSFunctionType(_) => "TSFunctionType",
    TSType::TSImportType(_) => "TSImportType",
    TSType::TSIndexedAccessType(_) => "TSIndexedAccessType",
    TSType::TSInferType(_) => "TSInferType",
    TSType::TSIntersectionType(_) => "TSIntersectionType",
    TSType::TSLiteralType(_) => "TSLiteralType",
    TSType::TSMappedType(_) => "TSMappedType",
    TSType::TSNamedTupleMember(_) => "TSNamedTupleMember",
    TSType::TSTemplateLiteralType(_) => "TSTemplateLiteralType",
    TSType::TSThisType(_) => "TSThisType",
    TSType::TSTupleType(_) => "TSTupleType",
    TSType::TSTypeLiteral(_) => "TSTypeLiteral",
    TSType::TSTypeOperatorType(_) => "TSTypeOperatorType",
    TSType::TSTypePredicate(_) => "TSTypePredicate",
    TSType::TSTypeQuery(_) => "TSTypeQuery",
    TSType::TSTypeReference(_) => "TSTypeReference",
    TSType::TSUnionType(_) => "TSUnionType",
    TSType::TSParenthesizedType(_) => "TSParenthesizedType",
    TSType::JSDocNullableType(_) => "JSDocNullableType",
    TSType::JSDocNonNullableType(_) => "JSDocNonNullableType",
    TSType::JSDocUnknownType(_) => "JSDocUnknownType",
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
      if let TSType::TSLiteralType(literal) = t
        && let TSLiteral::StringLiteral(value) = &literal.literal
      {
        trace!("Found value: {}", value.value.cyan());
        return Some(value.value.to_string());
      }
      None::<String>
    })
    .collect::<Vec<_>>();

  if found.is_empty() {
    #[cfg(debug_assertions)]
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
