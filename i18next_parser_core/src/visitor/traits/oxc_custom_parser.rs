use std::panic;

use color_eyre::owo_colors::OwoColorize;
use log::{error, trace, warn};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
  BindingPattern, BindingPatternKind, Declaration, Expression, ImportDeclarationSpecifier, ModuleDeclaration,
  ModuleExportName, ObjectPropertyKind, Statement, TSLiteral, TSSignature, TSType, TSTypeAnnotation, TSTypeName,
  TSUnionType,
};
use oxc_parser::Parser;
use oxc_resolver::ResolveContext;
use oxc_span::SourceType;
use serde_json::Value;

use crate::{
  helper::{MakeRelativePath, SerdeHelper, SerdeVecHelper},
  visitor::{
    parser::ModuleParser,
    traits::{GetLineBound, oxc_program::OxcProgram, print_error_location::PrintErrorLocation},
  },
};

pub trait OxcCustomParser: OxcProgram + PrintErrorLocation + GetLineBound {
  /// Find the value of an identifier in a statement
  fn find_value_for_identifier(&self, stmt: &Statement<'_>, identifier: &str) -> Option<Value> {
    if let Some(decl) = stmt.as_declaration() {
      self.parse_value_for_identifier_from_declaration(identifier, decl)
    } else if let Some(module) = stmt.as_module_declaration() {
      self.parse_value_from_module_declaration(identifier, module)
    } else {
      // #[cfg(debug_assertions)]
      // {
      //   warn!(
      //     "{} Unsupported statement: {stmt:?}",
      //     "[find_value_for_identifier]".red().bold(),
      //     stmt = stmt.bright_black().italic()
      //   );
      // }
      // #[cfg(not(debug_assertions))]
      // {
      //   debug!(
      //     "{} Unsupported statement: {}",
      //     "[find_value_for_identifier]".red().bold(),
      //     std::any::type_name_of_val(&stmt)
      //   );
      // }
      None
    }
  }

  fn find_value_for_identifier_from_start_declarations(&self, stmt: &Statement<'_>, identifier: &str) -> Option<Value> {
    if let Some(module) = stmt.as_module_declaration() {
      match module {
        ModuleDeclaration::ExportNamedDeclaration(_) => None,
        ModuleDeclaration::ImportDeclaration(_) => None,
        ModuleDeclaration::ExportAllDeclaration(declaration) => {
          self.find_value_identifier_and_declaration(declaration.source.value.as_str(), identifier)
        }
        ModuleDeclaration::ExportDefaultDeclaration(_) => None,
        _module => {
          #[cfg(debug_assertions)]
          warn!(
            "{} Unsupported module declaration: {module:?}",
            "[parse_value_from_module_declaration]".red().bold(),
            module = _module.bright_black().italic()
          );

          None
        }
      }
    } else {
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
    trace!(
      "Looking for identifier value: {} in {}",
      identifier.cyan(),
      self.file_path().make_relative(self.working_dir()).display().yellow()
    );

    let value = self
      .program()
      .body
      .iter()
      .find_map(|stmt| self.find_value_for_identifier(stmt, identifier));

    if let Some(value) = &value {
      trace!(
        "{} Found value for {}: {value:?}",
        "[find_identifier_value_as_serde]".blue(),
        identifier
      );
    } else {
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot find value of {name} in {}",
        "[find_identifier_value_as_serde]".red().bold(),
        self
          .file_path()
          .make_relative(self.working_dir())
          .display()
          .yellow()
          .dimmed(),
        name = identifier.cyan()
      );
      #[cfg(not(debug_assertions))]
      log::debug!(
        "{} Cannot find value of {name}",
        "[find_identifier_value_as_serde]".red().bold(),
        name = identifier.cyan()
      );
    }

    value
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
  fn parse_expression_as_serde(&self, expr: &Expression<'_>) -> Option<Value> {
    use serde_json::json;
    #[cfg(debug_assertions)]
    trace!(
      "{} Parsing expression to value: {:?}",
      "[parse_expression_as_serde]".blue().italic(),
      expr.bright_black().italic()
    );

    let ret = match expr {
      Expression::StringLiteral(str) => Some(json!(str.value.to_string())),
      Expression::NumericLiteral(num) => Some(json!(num.value.to_string())),
      Expression::BooleanLiteral(bool) => Some(json!(bool.value.to_string())),
      Expression::ArrayExpression(arr) => Some(Value::Array(
        arr
          .elements
          .iter()
          .map(|e| {
            if let Some(expession) = e.as_expression() {
              trace!("Parsing array expession");
              self.parse_expression_as_serde(expession).unwrap_or(Value::Null)
            } else {
              Value::Null
            }
          })
          .collect(),
      )),
      Expression::ObjectExpression(obj) => Some(Value::Object(
        obj
          .properties
          .iter()
          .filter_map(|prop| {
            if let ObjectPropertyKind::ObjectProperty(kv) = prop {
              trace!("Parsing object expression");
              let key = kv.key.name().unwrap_or_default().to_string();
              let value = self.parse_expression_as_serde(&kv.value);
              Some((key, value.unwrap_or(Value::Null)))
            } else {
              None
            }
          })
          .collect(),
      )),
      Expression::Identifier(identifier) => self.find_identifier_value_as_serde(&identifier.name),
      Expression::TSSatisfiesExpression(expr) => self
        .parse_expression_as_serde(&expr.expression)
        .or_else(|| self.parse_ts_type(&expr.type_annotation)),
      Expression::TSAsExpression(expression) => self
        .parse_expression_as_serde(&expression.expression)
        .or_else(|| self.parse_ts_type(&expression.type_annotation)),
      Expression::CallExpression(call) => self.parse_expression_as_serde(&call.callee),
      Expression::StaticMemberExpression(_static_member) => None,
      Expression::ConditionalExpression(condition) => {
        let consequent = self.parse_expression_as_serde(&condition.consequent);
        let alternate = self.parse_expression_as_serde(&condition.alternate);
        trace!("Consequent: {:?}", consequent);
        trace!("Alternate: {:?}", alternate);
        match (consequent, alternate) {
          (Some(consequent), Some(alternate)) => vec![consequent, alternate].values_to_value(),
          (Some(consequent), None) => Some(consequent),
          (None, Some(alternate)) => Some(alternate),
          (None, None) => None,
        }
      }
      _ => {
        #[cfg(debug_assertions)]
        {
          use oxc_span::GetSpan;

          warn!(
            "{} Unsupported expression: {}{} {expr:?}",
            "[parse_expression]".red().bold(),
            self
              .file_path()
              .make_relative(self.working_dir())
              .display()
              .yellow()
              .dimmed(),
            self.get_line_bounds(&expr.span()),
            expr = expr.bright_black().italic()
          );
          self.print_error_location(&expr.span());
        }
        None
      }
    };

    if let Some(ret) = &ret {
      trace!("{} Found value: {ret}", "[parse_expression_to_serde_value]".blue());
    } else {
      trace!(
        "{} found for expression: {:?}",
        "No value".red().bold(),
        expr.bright_black().italic()
      );
    }

    ret
  }

  fn parse_value_for_identifier_from_declaration(&self, identifier: &str, decl: &Declaration<'_>) -> Option<Value> {
    match decl {
      Declaration::VariableDeclaration(var) => {
        if let Some(item) = var
          .declarations
          .iter()
          .find(|e| e.id.get_identifier_name().is_some_and(|idx| idx.eq(identifier)))
        {
          item
            .init
            .as_ref()
            .and_then(|init| {
              trace!(
                "{} Parsing type annotation from init {:?}",
                "[VariableDeclaration]".blue(),
                init.bright_black().italic()
              );
              self.parse_expression_as_serde(init)
            })
            .or_else(|| {
              if let Some(type_annotation) = item.id.type_annotation.as_ref() {
                trace!(
                  "{} Parsing type annotation for item: {:?}",
                  "[VariableDeclaration]".blue(),
                  type_annotation.bright_black().italic()
                );
                let val = self.parse_ts_type(&type_annotation.type_annotation);
                if let Some(val) = &val {
                  trace!("{} Found value: {val}", "[VariableDeclaration]".blue());
                } else {
                  trace!(
                    "{} Cannot find value for identifier: {}",
                    "[VariableDeclaration]".red().bold(),
                    identifier.cyan()
                  );
                }
                val
              } else {
                None
              }
            })
        } else {
          var.declarations.iter().find_map(|e| {
            e.init.as_ref().and_then(|init| {
              if let Expression::ArrowFunctionExpression(arrow_function) = init {
                if let Some(type_annotation) = arrow_function
                  .params
                  .iter_bindings()
                  .find_map(|param| self.find_type_of_identifier(identifier, param))
                {
                  self.parse_ts_type(&type_annotation.type_annotation)
                } else {
                  None
                }
              } else {
                None
              }
            })
          })
        }
      }
      Declaration::FunctionDeclaration(func) => {
        if let Some(type_annotation) = func
          .params
          .iter_bindings()
          .find_map(|param| self.find_type_of_identifier(identifier, param))
        {
          self.parse_ts_type(&type_annotation.type_annotation)
        } else if let Some(type_annotation) = func.return_type.as_ref() {
          self.parse_ts_type(&type_annotation.type_annotation)
        } else if let Some(body) = func.body.as_ref() {
          body
            .statements
            .iter()
            .find_map(|stmt| self.find_value_for_identifier(stmt, identifier))
        } else {
          None
        }
      }
      Declaration::TSTypeAliasDeclaration(type_alias) if type_alias.id.name.eq(identifier) => {
        trace!(
          "{} Parsing type alias for {}: {:?}",
          "[TSTypeAliasDeclaration]".blue(),
          identifier.cyan(),
          type_alias.bright_black().italic()
        );
        self.parse_ts_type(&type_alias.type_annotation)
      }
      Declaration::TSTypeAliasDeclaration(_) => None,
      Declaration::TSInterfaceDeclaration(_) => None,
      _exported => {
        #[cfg(debug_assertions)]
        warn!(
          "Declaration of {identifier} is not supported {exported:?}",
          identifier = identifier.cyan().italic(),
          exported = _exported.bright_black().italic()
        );
        #[cfg(not(debug_assertions))]
        trace!(
          "Declaration of {identifier} is not supported {exported:?}",
          identifier = identifier.cyan().italic(),
          exported = std::any::type_name_of_val(&_exported)
        );
        None
      }
    }
  }

  fn parse_ts_type(&self, ts_type: &TSType<'_>) -> Option<Value> {
    trace!(
      "{} Parsing type: {:?}",
      "[parse_ts_type]".blue(),
      ts_type.bright_black().italic()
    );

    match &ts_type {
      TSType::TSLiteralType(literal) => get_value_from_literal_type(literal),
      TSType::TSUnionType(union_type) => get_string_values_from_union_type_literal(union_type)
        .map(|v| Value::Array(v.iter().map(|val| Value::String(val.clone())).collect())),
      TSType::TSTypeQuery(type_query) => type_query
        .expr_name
        .as_ts_type_name()
        .and_then(|type_name| self.parse_ts_type_name(type_name)),
      TSType::TSParenthesizedType(parenthesized) => self.parse_ts_type(&parenthesized.type_annotation),
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_const_type_reference() => None,
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_keyword() => {
        trace!(
          "{} Parsing indexed object type {:?}",
          "[parse_ts_type]".blue(),
          indexed.object_type.bright_black().italic()
        );
        self.parse_ts_type(&indexed.object_type)
      }
      TSType::TSIndexedAccessType(indexed) if indexed.index_type.is_keyword_or_literal() => {
        trace!(
          "{} Parsing index {:?}",
          "[parse_ts_type]".blue(),
          indexed.object_type.bright_black().italic()
        );
        self
          .parse_ts_type(&indexed.index_type)
          .value_to_string()
          .and_then(|key| {
            trace!(
              "{} Looking for {} in {:?}",
              "[parse_ts_type]".blue(),
              key.cyan(),
              indexed.object_type.bright_black().italic()
            );
            let value = self.parse_ts_type(&indexed.object_type);
            if let Some(Value::Object(value)) = value {
              value.get(&key).cloned()
            } else {
              None
            }
          })
      }
      TSType::TSArrayType(array_type) => self.parse_ts_type(&array_type.element_type),
      TSType::TSTypeReference(type_reference) if type_reference.type_name.is_const() => None,
      TSType::TSTypeReference(type_reference) => match &type_reference.type_name {
        TSTypeName::IdentifierReference(identifier) if identifier.name.eq("Array") => {
          type_reference.type_arguments.as_ref().and_then(|type_args| {
            if type_args.params.len() == 1 {
              self.parse_ts_type(type_args.params.first().expect("only 1 argument"))
            } else {
              self.parse_ts_type_name(&type_reference.type_name)
            }
          })
        }
        _ => self.parse_ts_type_name(&type_reference.type_name),
      },
      TSType::TSTupleType(tuple) => tuple
        .element_types
        .iter()
        .filter_map(|e| e.as_ts_type())
        .filter_map(|t| self.parse_ts_type(t))
        .collect::<Vec<_>>()
        .values_to_value(),
      TSType::TSTypeOperatorType(operator) => self.parse_ts_type(&operator.type_annotation),
      TSType::TSTypeLiteral(type_literal) => Some(Value::Object(
        type_literal
          .members
          .iter()
          .filter_map(|signature| match signature {
            TSSignature::TSPropertySignature(property) => property
              .type_annotation
              .as_ref()
              .and_then(|v| self.parse_ts_type(&v.type_annotation))
              .map(|value| (property.key.name().unwrap_or_default().to_string(), value)),
            _ => None,
          })
          .collect(),
      )),
      TSType::TSIndexedAccessType(_) => None,
      TSType::TSTypePredicate(_) => None,
      _ => {
        #[cfg(debug_assertions)]
        {
          use oxc_span::GetSpan;

          warn!(
            "{} Unsupported type annotation in {}{}: {ts_type:?}",
            "[parse_ts_type]".red().bold(),
            self
              .file_path()
              .make_relative(self.working_dir())
              .display()
              .yellow()
              .dimmed(),
            self.get_line_bounds(&ts_type.span()).blue(),
            ts_type = ts_type.bright_black().italic()
          );
        }
        #[cfg(not(debug_assertions))]
        trace!(
          "{} Unsupported type annotation: {ts_type}",
          "[parse_ts_type]".red().bold(),
          ts_type = std::any::type_name_of_val(&ts_type)
        );
        None
      }
    }
  }

  fn parse_ts_type_name(&self, type_name: &oxc_ast::ast::TSTypeName<'_>) -> Option<Value> {
    if let TSTypeName::IdentifierReference(identifier) = &type_name {
      trace!(
        "{} Looking for type reference {} {:?}",
        "[parse_ts_type_name]".blue(),
        identifier.name.cyan(),
        type_name.bright_black()
      );

      let parse_type_from_declaration = |decl: &Declaration| {
        if let Declaration::VariableDeclaration(var) = decl {
          if var.declarations.iter().any(|decl| {
            decl
              .id
              .get_identifier_name()
              .is_some_and(|name| name.eq(&identifier.name))
          }) {
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
        stmt
          .as_declaration()
          .and_then(parse_type_from_declaration)
          .or_else(|| match stmt {
            Statement::TSTypeAliasDeclaration(type_alias) if type_alias.id.name == identifier.name => {
              match &type_alias.type_annotation {
                TSType::TSTypeReference(type_reference) if type_reference.type_name.eq(type_name) => {
                  log::trace!(
                    "Skipping type {type_name} to avoid infinite loop",
                    type_name = type_name.bright_black()
                  );
                  None
                }
                _ => self.parse_ts_type(&type_alias.type_annotation),
              }
            }
            Statement::ImportDeclaration(import_decl) => {
              let result = import_decl.specifiers.as_ref().and_then(|specifiers| {
                specifiers.iter().find_map(|specifier| match specifier {
                  ImportDeclarationSpecifier::ImportSpecifier(specifier)
                    if specifier.local.name.eq(&identifier.name) =>
                  {
                    match &specifier.imported {
                      ModuleExportName::IdentifierReference(identifier_reference) => self
                        .find_value_identifier_and_declaration(&import_decl.source.value, &identifier_reference.name),
                      ModuleExportName::IdentifierName(name) => {
                        trace!(
                          "{} Identifier name: {:?}",
                          "[parse_ts_type_name]".blue().bold(),
                          name.name.cyan()
                        );

                        self.find_value_identifier_and_declaration(&import_decl.source.value, &name.name)
                      }
                      _ => {
                        #[cfg(debug_assertions)]
                        warn!(
                          "{} Unsupported import specifier: {:?}",
                          "[parse_ts_type_name]".red().bold(),
                          specifier.imported.bright_black()
                        );
                        #[cfg(not(debug_assertions))]
                        log::debug!(
                          "{} Unsupported import specifier: {}",
                          "[parse_ts_type_name]".red().bold(),
                          std::any::type_name_of_val(&specifier.imported)
                        );
                        None
                      }
                    }
                  }
                  _ => None,
                })
              });

              #[cfg(debug_assertions)]
              {
                use oxc_span::GetSpan;
                if result.is_none()
                  && import_decl
                    .specifiers
                    .as_ref()
                    .is_some_and(|spec| spec.iter().any(|s| s.name().eq(&identifier.name)))
                {
                  log::warn!(
                    "{} {file} Value of identifier {} is not exported in import declaration",
                    "[parse_ts_type_name]".red().bold(),
                    identifier.name.cyan(),
                    file = self
                      .file_path()
                      .make_relative(self.working_dir())
                      .display()
                      .yellow()
                      .dimmed()
                  );
                  self.print_error_location(&identifier.span());
                }
              }

              result
            }
            Statement::VariableDeclaration(_) => None,
            Statement::TSTypeAliasDeclaration(_) => None,
            Statement::ExportNamedDeclaration(export) if export.declaration.is_some() => {
              let decl = export.declaration.as_ref().unwrap();
              self.parse_value_for_identifier_from_declaration(&identifier.name, decl)
            }
            Statement::ExportNamedDeclaration(_) => None,
            Statement::ExportAllDeclaration(_) => None,
            Statement::ExportDefaultDeclaration(_) => None,
            Statement::FunctionDeclaration(_) => None,
            Statement::ExpressionStatement(_) => None,
            Statement::TSInterfaceDeclaration(_) => None,
            _statement => {
              warn!(
                "{} Unsupported statement: {statement:?} [Declaration Type: {is_declaration}]",
                "[parse_ts_type_name]".red().bold(),
                statement = _statement.bright_black().italic(),
                is_declaration = _statement.is_declaration()
              );
              None
            }
          })
      });

      if let Some(val) = &val {
        log::trace!(
          "{} Found value {} = {} in {}",
          "[parse_ts_type_name]".on_green().black().bold(),
          identifier.name.cyan(),
          val.purple(),
          self
            .file_path()
            .make_relative(self.working_dir())
            .display()
            .yellow()
            .dimmed()
        );
      } else {
        log::trace!(
          "{} Cannot find value for identifier: {:?} in {}",
          "[parse_ts_type_name]".on_red().black().bold(),
          identifier.name.cyan(),
          self
            .file_path()
            .make_relative(self.working_dir())
            .display()
            .yellow()
            .dimmed()
        );
      }

      val
    } else {
      // #[cfg(debug_assertions)]
      // warn!(
      //   "{} Unsupported type name: {type_name:?}",
      //   "[parse_ts_type_name]".red().bold(),
      //   type_name = type_name.bright_black().italic()
      // );
      // #[cfg(not(debug_assertions))]
      // debug!(
      //   "{} Unsupported type name: {}",
      //   "[parse_ts_type_name]".red().bold(),
      //   std::any::type_name_of_val(type_name)
      // );
      None
    }
  }

  /// Parse a value from a module declaration
  fn parse_value_from_module_declaration(&self, identifier: &str, module: &ModuleDeclaration<'_>) -> Option<Value> {
    match module {
      ModuleDeclaration::ExportNamedDeclaration(decl) => decl.declaration.as_ref().and_then(|declaration| {
        log::trace!(
          "{} Looking for identifier {identifier:?} in declaration: {declaration:?}",
          "[ExportNamedDeclaration]".blue(),
          identifier = identifier.cyan(),
          declaration = declaration.bright_black().italic(),
        );

        let result = self.parse_value_for_identifier_from_declaration(identifier, declaration);
        if let Some(result) = &result {
          log::trace!(
            "{} Found: {result}",
            "[ExportNamedDeclaration]".blue(),
            result = result.yellow()
          );
        } else {
          log::trace!(
            "{} No value found for {identifier}",
            "[ExportNamedDeclaration]".blue(),
            identifier = identifier.cyan()
          );
        }
        result
      }),
      ModuleDeclaration::ImportDeclaration(import)
        if import
          .specifiers
          .as_ref()
          .is_some_and(|specifiers| specifiers.iter().any(|s| s.name().eq(&identifier))) =>
      {
        log::trace!("Finding value from import declaration");
        self.find_value_identifier_and_declaration(&import.source.value, identifier)
      }
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
      }
      ModuleDeclaration::ExportAllDeclaration(_) => None,
      ModuleDeclaration::ExportDefaultDeclaration(_) => None,
      _module => {
        #[cfg(debug_assertions)]
        warn!(
          "{} Unsupported module declaration: {module:?}",
          "[parse_value_from_module_declaration]".red().bold(),
          module = _module.bright_black().italic()
        );
        #[cfg(not(debug_assertions))]
        {
          let _module = match _module {
            ModuleDeclaration::ImportDeclaration(_) => "ImportDeclaration",
            ModuleDeclaration::ExportAllDeclaration(_) => "ExportAllDeclaration",
            ModuleDeclaration::ExportDefaultDeclaration(_) => "ExportDefaultDeclaration",
            ModuleDeclaration::ExportNamedDeclaration(_) => "ExportNamedDeclaration",
            ModuleDeclaration::TSExportAssignment(_) => "TSExportAssignment",
            ModuleDeclaration::TSNamespaceExportDeclaration(_) => "TSNamespaceExportDeclaration",
          };
          log::debug!(
            "{} Unsupported module declaration: {}",
            "[parse_value_from_module_declaration]".red().bold(),
            _module
          );
        }

        None
      }
    }
  }

  fn find_value_identifier_and_declaration(&self, path_to_resolve: &str, identifier: &str) -> Option<Value> {
    let path = (if self.file_path().is_dir() {
      Some(self.file_path().as_path())
    } else {
      self.file_path().parent()
    })
    .unwrap();
    let path_to_resolve = if path_to_resolve == "." {
      warn!("Resolving path {path_to_resolve} as index");
      "index"
    } else {
      path_to_resolve
    };
    if path == path_to_resolve {
      warn!("Attempting to resolve the same path {path_to_resolve} from itself, which may lead to infinite recursion.");
      return None;
    }

    trace!(
      "{} resolving {} from {}",
      "[find_value_identifier_and_declaration]".on_yellow().black().bold(),
      path_to_resolve.yellow(),
      path.display().yellow()
    );
    let mut ctx = ResolveContext::default();
    let resolved = self.resolver().resolve_with_context(path, path_to_resolve, &mut ctx);
    match resolved {
      Ok(resolved) => {
        if resolved
          .path()
          .as_os_str()
          .to_str()
          .is_some_and(|s| s.contains("node_modules"))
        {
          trace!(
            "Skipping the resolution of node_modules file {}",
            resolved.path().display().yellow()
          );
          return None;
        }
        trace!(
          "{} resolved to {}",
          "[find_value_identifier_and_declaration]".on_green().black().bold(),
          resolved.path().display().yellow()
        );
        trace!("Resolved: {:#?}", resolved.bright_black().dimmed());
        trace!("Resolution context: {:#?}", ctx.bright_black().dimmed());

        let path = resolved.path();
        let source_type = SourceType::from_path(path).unwrap();
        let source_text = std::fs::read_to_string(path).unwrap();
        trace!(
          "{} Parsing file {}",
          "[find_value_identifier_and_declaration]".on_yellow().black().bold(),
          path.display().yellow()
        );
        let parser = Parser::new(self.allocator(), source_text.as_str(), source_type);
        let result = parser.parse();
        if result.panicked {
          warn!(
            "{} Failed to parse file {}: {:?}",
            "[find_value_identifier_and_declaration]".on_red().black().bold(),
            path.display().yellow(),
            result.errors
          );
          return None;
        }
        trace!(
          "{} {} parsed successfully",
          "[find_value_identifier_and_declaration]".on_green().black().bold(),
          path.display().yellow()
        );

        let allocator = Default::default();
        let module_parser = ModuleParser::new(&result.program, &allocator, path.into(), self.working_dir());

        trace!(
          "{} Looking for {} in {}",
          "[find_value_identifier_and_declaration]".on_yellow().black().bold(),
          identifier.cyan(),
          path.display().yellow()
        );
        let val = result
          .program
          .body
          .iter()
          .find_map(|stmt| module_parser.find_value_for_identifier(stmt, identifier))
          .or_else(|| {
            trace!(
              "{} Looking into every exported * modules for {}",
              "[find_value_identifier_and_declaration]".on_yellow().black().bold(),
              identifier.cyan(),
            );
            result
              .program
              .body
              .iter()
              .filter(|stmt| stmt.is_module_declaration())
              .find_map(|stmt| module_parser.find_value_for_identifier_from_start_declarations(stmt, identifier))
          });
        if let Some(val) = &val {
          trace!(
            "{} Found value {} for {} in {}",
            "[find_value_identifier_and_declaration]".on_green().black().bold(),
            identifier.cyan(),
            val.purple(),
            path.display().yellow()
          );
        } else {
          trace!(
            "{} {} not found in {}",
            "[find_value_identifier_and_declaration]".on_red().black().bold(),
            identifier.cyan(),
            path.display().yellow()
          )
        }
        val
      }
      Err(err) => {
        if cfg!(debug_assertions) {
          error!(
            "Resolver options: {:#?}",
            self.resolver().options().bright_black().italic()
          );

          panic!(
            "{} Failed to resolve import to {} from {}: {}",
            "[find_value_identifier_and_declaration]".on_red().bold(),
            path_to_resolve.yellow(),
            path.display().yellow(),
            err.red()
          );
        }

        None
      }
    }
  }

  fn resolve_remote_type(
    &self,
    path_to_resolve: &str,
    identifier: &str,
  ) -> Option<oxc_allocator::Box<'_, TSTypeAnnotation<'_>>> {
    let path = (if self.file_path().is_dir() {
      Some(self.file_path().as_path())
    } else {
      self.file_path().parent()
    })
    .unwrap();
    let path_to_resolve = if path_to_resolve == "." {
      warn!("Resolving path {path_to_resolve} as index");
      "index"
    } else {
      path_to_resolve
    };

    trace!(
      "{} resolving {} from {}",
      "[find_value_identifier_and_declaration]".on_yellow().black().bold(),
      path_to_resolve.yellow(),
      path.display().yellow()
    );
    let mut ctx = ResolveContext::default();
    let resolved = self.resolver().resolve_with_context(path, path_to_resolve, &mut ctx);
    match resolved {
      Ok(resolved) => {
        trace!(
          "{} resolved to {}",
          "[resolve_remote_type]".on_green().black().bold(),
          resolved.path().display().yellow()
        );
        trace!("Resolved: {:#?}", resolved.bright_black().dimmed());
        trace!("Resolution context: {:#?}", ctx.bright_black().dimmed());

        let path = resolved.path();
        let source_type = SourceType::from_path(path).unwrap();
        let source_text = std::fs::read_to_string(path).unwrap();
        let source_text = self.allocator().alloc_str(&source_text);
        trace!(
          "{} Parsing file {}",
          "[parse_i18next_option]".on_yellow().black().bold(),
          path.display().yellow()
        );

        let parser = Parser::new(self.allocator(), source_text, source_type);
        let result = parser.parse();
        if result.panicked {
          warn!(
            "{} Failed to parse file {}: {:?}",
            "[resolve_remote_type]".on_red().black().bold(),
            path.display().yellow(),
            result.errors
          );
          return None;
        }
        trace!(
          "{} {} parsed successfully",
          "[resolve_remote_type]".on_green().black().bold(),
          path.display().yellow()
        );
        let program = self.allocator().alloc(result.program);

        let module_parser = ModuleParser::new(program, self.allocator(), path.into(), self.working_dir());

        trace!(
          "{} Looking for {} in {}",
          "[resolve_remote_type]".on_yellow().black().bold(),
          identifier.cyan(),
          path.display().yellow()
        );
        let val = module_parser
          .find_remote_type(identifier)
          .map(|t| t.clone_in(self.allocator()));

        if let Some(val) = &val {
          trace!(
            "{} Found value {} = {:?} in {}",
            "[find_value_identifier_and_declaration]".on_green().black().bold(),
            identifier.cyan(),
            val.purple(),
            path.display().yellow()
          );
        } else {
          trace!(
            "{} {} not found in {}",
            "[find_value_identifier_and_declaration]".on_red().black().bold(),
            identifier.cyan(),
            path.display().yellow()
          )
        }

        val
      }
      Err(err) => {
        if cfg!(debug_assertions) {
          error!(
            "Resolver options: {:#?}",
            self.resolver().options().bright_black().italic()
          );

          panic!(
            "{} Failed to resolve import to {} from {}: {}",
            "[resolve_remote_type]".on_red().bold(),
            path_to_resolve.yellow(),
            path.display().yellow(),
            err.red()
          );
        }

        None
      }
    }
  }

  fn find_remote_type(&self, identifier: &str) -> Option<oxc_allocator::Box<'_, TSTypeAnnotation<'_>>> {
    self.program().body.iter().find_map(|stmt| {
      if let Some(decl) = stmt.as_declaration() {
        match decl {
          Declaration::VariableDeclaration(_var) => None,
          Declaration::FunctionDeclaration(_func) => None,
          Declaration::TSTypeAliasDeclaration(type_alias) if type_alias.id.name.eq(identifier) => {
            // Instead of returning a reference tied to the local `module_parser`, clone the TSTypeAnnotation if found.
            self.find_type_of_identifier_from_ts_type(identifier, &type_alias.type_annotation)
          }
          Declaration::TSTypeAliasDeclaration(_) => None,
          Declaration::TSInterfaceDeclaration(_) => None,
          _exported => {
            #[cfg(debug_assertions)]
            log::warn!(
              "Declaration of {identifier} is not supported {exported:?}",
              identifier = identifier.cyan().italic(),
              exported = _exported.bright_black().italic()
            );
            #[cfg(not(debug_assertions))]
            log::trace!(
              "Declaration of {identifier} is not supported {exported:?}",
              identifier = identifier.cyan().italic(),
              exported = std::any::type_name_of_val(&_exported)
            );
            None
          }
        }
      } else {
        None
      }
    })
  }

  fn find_type_of_identifier(
    &self,
    identifier: &str,
    param: &BindingPattern<'_>,
  ) -> Option<oxc_allocator::Box<'_, TSTypeAnnotation<'_>>> {
    trace!(
      "{} Looking for {} in BindingPattern",
      "[find_type_of_identifier]".blue().bold(),
      identifier.cyan()
    );
    match &param.kind {
      BindingPatternKind::BindingIdentifier(idx) if idx.name.eq(&identifier) => {
        param.type_annotation.clone_in(self.allocator())
      }
      BindingPatternKind::ObjectPattern(obj) => {
        let property = obj
          .properties
          .iter()
          .find(|prop| prop.key.name().is_some_and(|name| name.eq(&identifier)));
        if let Some(prop) = property {
          trace!(
            "Found key {} in {:?}",
            identifier.cyan(),
            obj.properties.bright_black().dimmed()
          );
          prop
            .value
            .type_annotation
            .clone_in(self.allocator())
            .or(param.type_annotation.as_ref().and_then(|type_annotation| {
              self.find_type_of_identifier_from_ts_type(identifier, &type_annotation.type_annotation)
            }))
        } else {
          None
        }
      }
      _ => None,
    }
  }

  fn find_type_of_identifier_from_ts_type(
    &self,
    identifier: &str,
    ts_type: &TSType<'_>,
  ) -> Option<oxc_allocator::Box<'_, TSTypeAnnotation<'_>>> {
    trace!(
      "{} Looking for {} in {:?}",
      "[find_type_of_identifier_from_ts_type]".blue(),
      identifier.cyan(),
      ts_type.bright_black().italic()
    );

    match ts_type {
      TSType::TSTypeLiteral(type_literal) => type_literal.members.iter().find_map(|member| match member {
        TSSignature::TSPropertySignature(signature)
          if signature.key.name().is_some_and(|name| name.eq(&identifier)) =>
        {
          signature.type_annotation.clone_in(self.allocator())
        }
        TSSignature::TSPropertySignature(_) => None,
        _ => {
          #[cfg(debug_assertions)]
          {
            use oxc_span::GetSpan;
            warn!(
              "{} Unsupported type annotation in object pattern: {type_annotation:?}",
              "[find_type_of_identifier_from_ts_type]".red().bold(),
              type_annotation = ts_type.bright_black().italic()
            );
            self.print_error_location(&member.span());
          }
          #[cfg(not(debug_assertions))]
          log::debug!(
            "{} Unsupported type annotation in object pattern: {}",
            "[find_type_of_identifier_from_ts_type]".red().bold(),
            get_ts_type_name(ts_type)
          );
          None
        }
      }),
      TSType::TSIntersectionType(type_intersection) => {
        // i'm not sure if this is correct, but it seems to be the right way to handle intersections
        type_intersection
          .types
          .iter()
          .find_map(|t| self.find_type_of_identifier_from_ts_type(identifier, t))
      }
      TSType::TSUnionType(type_union) => {
        #[cfg(feature = "union")]
        {
          let mut a = std::collections::HashMap::<String, Vec<&oxc_allocator::Box<'_, TSTypeAnnotation<'_>>>>::new();
          let span = type_union.span;
          for union in type_union.types.iter() {
            if let TSType::TSTypeLiteral(literal) = union {
              literal.members.iter().for_each(|member| {
                if let TSSignature::TSPropertySignature(tsproperty_signature) = member {
                  if let Some(name) = tsproperty_signature.key.name() {
                    if let Some(type_annotation) = &tsproperty_signature.type_annotation {
                      let name = name.into_owned();
                      if let Some(value) = a.get_mut(&name) {
                        value.push(type_annotation);
                      } else {
                        a.insert(name, vec![type_annotation]);
                      }
                    }
                  }
                } else {
                  todo!()
                }
              });
            } else {
              error!("Not ts literal type");
            }
          }

          a.iter().for_each(|(key, value)| trace!("{} {:#?}", key, value));
        }
        type_union
          .types
          .iter()
          .find_map(|t| self.find_type_of_identifier_from_ts_type(identifier, t))
      }
      TSType::TSTypeReference(type_reference) if type_reference.type_arguments.is_some() => {
        type_reference.type_arguments.as_ref().and_then(|args| {
          args
            .params
            .iter()
            .find_map(|t| self.find_type_of_identifier_from_ts_type(identifier, t))
        })
      }
      TSType::TSTypeReference(type_reference) => {
        if let TSTypeName::IdentifierReference(type_identifier) = &type_reference.type_name {
          self.program().body.iter().find_map(|stmt| match stmt {
            Statement::TSTypeAliasDeclaration(type_alias) if type_alias.id.name == type_identifier.name => {
              self.find_type_of_identifier_from_ts_type(identifier, &type_alias.type_annotation)
            }
            Statement::ImportDeclaration(import_decl) => {
              let result = import_decl.specifiers.as_ref().and_then(|specifiers| {
                specifiers.iter().find_map(|specifier| match specifier {
                  ImportDeclarationSpecifier::ImportSpecifier(specifier)
                    if specifier.local.name.eq(&type_identifier.name) =>
                  {
                    match &specifier.imported {
                      ModuleExportName::IdentifierReference(identifier_reference) => {
                        self.resolve_remote_type(&import_decl.source.value, &identifier_reference.name)
                      }
                      ModuleExportName::IdentifierName(name) => {
                        trace!("Identifier name: {:?}", name.name.cyan());
                        self.resolve_remote_type(&import_decl.source.value, &name.name)
                      }
                      _ => {
                        #[cfg(debug_assertions)]
                        warn!(
                          "{} Unsupported import specifier: {:?}",
                          "[parse_ts_type_name]".red().bold(),
                          specifier.imported.bright_black()
                        );
                        #[cfg(not(debug_assertions))]
                        log::debug!(
                          "{} Unsupported import specifier: {}",
                          "[parse_ts_type_name]".red().bold(),
                          std::any::type_name_of_val(&specifier.imported)
                        );
                        None
                      }
                    }
                  }
                  _ => None,
                })
              });

              #[cfg(debug_assertions)]
              {
                use oxc_span::GetSpan;
                if result.is_none()
                  && import_decl
                    .specifiers
                    .as_ref()
                    .is_some_and(|spec| spec.iter().any(|s| s.name().eq(&type_identifier.name)))
                {
                  log::warn!(
                    "{} {file} Value of identifier {} is not exported in import declaration",
                    "[parse_ts_type_name]".red().bold(),
                    type_identifier.cyan(),
                    file = self
                      .file_path()
                      .make_relative(self.working_dir())
                      .display()
                      .yellow()
                      .dimmed()
                  );
                  self.print_error_location(&type_identifier.span());
                }
              }

              result
            }
            Statement::VariableDeclaration(_) => None,
            Statement::TSTypeAliasDeclaration(_) => None,
            Statement::ExportNamedDeclaration(_) => None,
            Statement::ExportAllDeclaration(_) => None,
            Statement::ExportDefaultDeclaration(_) => None,
            Statement::FunctionDeclaration(_) => None,
            Statement::ExpressionStatement(_) => None,
            Statement::TSInterfaceDeclaration(_) => None,
            _statement => {
              #[cfg(debug_assertions)]
              warn!(
                "{} Unsupported statement: {statement:?}",
                "[parse_ts_type_name]".red().bold(),
                statement = _statement.bright_black().italic()
              );
              #[cfg(not(debug_assertions))]
              log::debug!(
                "{} Unsupported statement {}",
                "[parse_ts_type_name]".red().bold(),
                std::any::type_name_of_val(_statement)
              );
              None
            }
          })
        } else {
          None
        }
      }
      _ => {
        let get_identifier_reference = ts_type.get_identifier_reference();
        if get_identifier_reference.is_none() || get_identifier_reference.is_some_and(|f| f.name.eq(identifier)) {
          #[cfg(debug_assertions)]
          {
            warn!(
              "{} Unable to find type of identifier {} from {type_annotation:?}",
              "[find_type_of_identifier_from_ts_type]".red().bold(),
              identifier.cyan(),
              type_annotation = ts_type.bright_black().italic()
            );
            use oxc_span::GetSpan;
            self.print_error_location(&ts_type.span());
          }
          #[cfg(not(debug_assertions))]
          log::debug!(
            "{} Unable to find type of identifier {} from {type_annotation}",
            "[find_type_of_identifier_from_ts_type]".red().bold(),
            identifier.cyan(),
            type_annotation = get_ts_type_name(ts_type).bold()
          );
        }

        None
      }
    }
  }
}

fn get_value_from_literal_type(literal: &oxc_allocator::Box<'_, oxc_ast::ast::TSLiteralType<'_>>) -> Option<Value> {
  match &literal.literal {
    TSLiteral::BooleanLiteral(boolean_literal) => Some(Value::Bool(boolean_literal.value)),
    TSLiteral::NumericLiteral(numeric_literal) => {
      serde_json::Number::from_f64(numeric_literal.value).map(Value::Number)
    }
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
      }
      (TSTypeName::QualifiedName(a), TSTypeName::QualifiedName(b)) => a.span == b.span,
      _ => false,
    }
  }
}
