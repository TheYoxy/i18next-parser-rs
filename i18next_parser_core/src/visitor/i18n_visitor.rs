use std::{collections::HashMap, path::PathBuf};

use color_eyre::owo_colors::OwoColorize;
use log::{debug, error, trace, warn};
use oxc_ast::ast::{
  self,
  Argument,
  BindingPattern,
  BindingPatternKind,
  CallExpression,
  Expression,
  IdentifierName,
  IdentifierReference,
  ImportDeclarationSpecifier,
  JSXAttributeItem,
  JSXAttributeName,
  JSXAttributeValue,
  JSXChild,
  JSXElement,
  JSXElementName,
  JSXExpression,
  ObjectExpression,
  ObjectPropertyKind,
  Program,
  PropertyKey,
  Statement,
  TSLiteral,
  TSType,
  TSTypeName,
};
use oxc_span::GetSpan;
use serde_json::Value;
use tracing::span;

use crate::{
  clean_multi_line_code,
  visitor::node_child::{NodeChild, NodeTag},
  Config,
  Entry,
  IsEmpty,
  Location,
};

/// This type alias represents the options for i18next.
/// It is a HashMap where the key is a String representing the option name,
/// and the value is an Option<`String`> representing the option value.
pub type I18NextOptions = HashMap<String, Option<String>>;

/// This struct represents the options for the I18NVisitor.
///
/// # Fields
///
/// * `namespace_separator` - The spearator to use for the namespace inside a key.
/// * `trans_keep_basic_html_nodes_for` - An optional vector of strings representing the basic HTML nodes to be kept for translation.
#[derive(Debug, Default)]
pub struct VisitorOptions {
  pub namespace_separator: Option<String>,
  pub trans_keep_basic_html_nodes_for: Option<Vec<String>>,
}

impl VisitorOptions {
  pub fn new<C: AsRef<Config>>(config: C) -> Self {
    let config = config.as_ref();
    VisitorOptions { namespace_separator: Some(config.namespace_separator.clone()), ..Default::default() }
  }
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
  pub fn new<Path: Into<PathBuf>, C: AsRef<Config>>(program: &'a Program<'a>, file_path: Path, config: C) -> Self {
    I18NVisitor {
      program,
      file_path: file_path.into(),
      entries: Default::default(),
      options: VisitorOptions::new(config),
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
      Expression::Identifier(identifier) => self.find_identifier_value(identifier),
      Expression::TSSatisfiesExpression(expr) => self.parse_expression(&expr.expression),
      Expression::TSAsExpression(expression) => self.parse_expression(&expression.expression),
      _ => {
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
      Expression::TSAsExpression(expression) => self.parse_expression_as_string(&expression.expression),
      _ => {
        #[cfg(debug_assertions)]
        warn!("{} Unsupported expression (str): {expr:?}", "[Parse_expression_as_string]".red().bold());
        #[cfg(not(debug_assertions))]
        warn!("{} Unsupported expression: {expr:?}", "[Parse_expression_as_string]".red().bold());
        None
      },
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
    let arr = self.program.body.iter().find_map(|stmt| {
      if let Statement::VariableDeclaration(var) = stmt {
        var
          .declarations
          .iter()
          .find(|v| v.id.get_identifier_name() == Some(identifier.name))
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
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot find str value of {name} in {path} {identifier:?}",
        "[Find_identifier_value_as_string_from_identifier_name]".red().bold(),
        path = self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
      #[cfg(not(debug_assertions))]
      warn!(
        "{} Cannot find str value of {name} in {path} {identifier:?}",
        "[Find_identifier_value_as_string_from_identifier_name]".red().bold(),
        path = self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
    }

    arr
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
    debug!("Looking for identifier value: {}", identifier.name);
    let arr = self.program.body.iter().find_map(|stmt| {
      if let Statement::VariableDeclaration(var) = stmt {
        var
          .declarations
          .iter()
          .find(|v| v.id.get_identifier_name() == Some(identifier.name))
          .and_then(|item| item.init.as_ref())
          .and_then(|init| self.parse_expression(init))
      } else {
        None
      }
    });

    if arr.is_none() {
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot value of {name} in {path} {identifier:?}",
        "[Find_identifier_value]".red().bold(),
        path = self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );

      #[cfg(not(debug_assertions))]
      warn!(
        "{} Cannot value of {name} in {path} {identifier:?}",
        "[Find_identifier_value]".red().bold(),
        path = self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
    }

    arr
  }

  fn find_identifier_value_as_vec_string(
    &self,
    identifier: &oxc_allocator::Box<IdentifierReference>,
  ) -> Option<Vec<String>> {
    fn fun_name<'a>(
      stmt: &Statement<'a>,
      identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
      this: &I18NVisitor<'a>,
    ) -> Option<Vec<String>> {
      debug!("Statement: {:#?}", stmt);
      match stmt {
        Statement::VariableDeclaration(var) => {
          debug!("Declarations {:#?}", var.declarations);
          var
            .declarations
            .iter()
            .find(|e| e.id.get_identifier_name().is_some_and(|idx| identifier.name.eq(&idx)))
            .and_then(|item| {
              debug!("Parsing item: {:#?}", item);
              item
                .init
                .as_ref()
                .and_then(|init| {
                  debug!("Parsing item value: {:#?}", init);
                  this.parse_expression_as_string(init).map(|v| vec![v])
                })
                .or_else(|| {
                  debug!("Parsing type annotation for item: {:#?}", item);
                  item
                    .id
                    .type_annotation
                    .as_ref()
                    .and_then(|type_annotation| this.parse_type_annotation_as_vec_str(type_annotation))
                })
            })
        },
        Statement::FunctionDeclaration(func)
          if func.params.iter_bindings().any(|param| find_type_of_identifier(identifier, param).is_some()) =>
        {
          func.params.iter_bindings().find_map(|param| find_type_of_identifier(identifier, param)).and_then(
            |type_annotation| {
              debug!("Type annotation: {:#?}", type_annotation);
              this.parse_type_annotation_as_vec_str(type_annotation)
            },
          )
        },
        Statement::FunctionDeclaration(func) if func.body.is_some() => {
          func.body.as_ref().unwrap().statements.iter().find_map(|stmt| fun_name(stmt, identifier, this))
        },
        _ => None,
      }
    }
    let arr = self.program.body.iter().find_map(|stmt| fun_name(stmt, identifier, self));

    if arr.is_none() {
      #[cfg(debug_assertions)]
      log::warn!(
        "{} Cannot vec values of {name} in {} {identifier:?}",
        "[Find_identifier_value_as_vec_string]".red().bold(),
        self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );

      #[cfg(not(debug_assertions))]
      log::warn!(
        "{} Cannot vec values of {name} in {}",
        "[Find_identifier_value_as_vec_string]".red().bold(),
        self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );
    }

    arr
  }

  fn parse_type_annotation_as_vec_str(
    &self,
    type_annotation: &oxc_allocator::Box<'_, ast::TSTypeAnnotation<'_>>,
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
          .find(|v| v.id.get_identifier_name().is_some_and(|name| name.eq(&identifier.name)))
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
      #[cfg(debug_assertions)]
      warn!(
        "{} Cannot str value of {name} in {} {identifier:?}",
        "[Find_identifier_value_as_string]".red().bold(),
        self.file_path.display().yellow(),
        name = identifier.name.cyan()
      );

      #[cfg(not(debug_assertions))]
      warn!(
        "{} Cannot str value of {name} in {}",
        "[Find_identifier_value_as_string]".red().bold(),
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
      "cloneInstance" => expr.arguments.first(),
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
        Argument::ObjectExpression(expression) => {
          let vec = expression
            .properties
            .iter()
            .filter_map(|prop| {
              if let ObjectPropertyKind::ObjectProperty(obj) = prop {
                match &obj.key {
                  PropertyKey::StringLiteral(str) if str.value == "ns" => {
                    return Some(str.value.to_string());
                  },
                  PropertyKey::StaticIdentifier(ident) if ident.name == "ns" => {
                    return self.find_identifier_value_as_string_from_identifier_name(ident);
                  },
                  PropertyKey::Identifier(ident) if ident.name == "ns" => {
                    return self.find_identifier_value_as_string(ident);
                  },
                  _ => (),
                }
                warn!("{} Unsupported property key: {:?}", "[Extract_namespace]".red().bold(), obj.key);
              }

              None
            })
            .collect::<Vec<String>>();
          let value = vec.first();
          self.current_namespace = value.cloned();
        },
        _ => {
          warn!("{} Unsupported argument for {name} {arg:?}", "[Extract_namespace]".red().bold(),);
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
          ObjectPropertyKind::SpreadProperty(prop) => {
            #[cfg(debug_assertions)]
            {
              use crate::visitor::visit::print_error_location;

              warn!("{} Unsupported spread property", "[Parse_i18next_option]".red().bold());
              print_error_location(&self.file_path, &prop.span).unwrap();
              panic!("Spread property is not supported in i18next options");
            }
            #[cfg(not(debug_assertions))]
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
  pub(super) fn get_prop_values(&self, elem: &JSXElement<'_>, attribute_name: &str) -> Option<Vec<String>> {
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
                    JSXAttributeValue::StringLiteral(str) => Some(vec![str.value.to_string()]),
                    JSXAttributeValue::ExpressionContainer(e) => {
                      // todo this expression will contains the required identifier
                      match &e.expression {
                        JSXExpression::StringLiteral(str) => Some(vec![str.value.to_string()]),
                        JSXExpression::Identifier(identifier) => {
                          trace!("Looking for identifier value for prop");
                          self.find_identifier_value_as_vec_string(identifier)
                        },
                        JSXExpression::NumericLiteral(num) => Some(vec![num.value.to_string()]),
                        JSXExpression::StaticMemberExpression(expression) => {
                          self.parse_expression_as_string(&expression.object).map(|v| vec![v])
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
      (Some(Argument::StringLiteral(str)), Some(Argument::Identifier(identifier))) => {
        let value = str.value.to_string();
        trace!("translation value defined as string literal: {}", value.cyan());
        warn!("The 3rd argument of t is an identifier. This is not supported and will be ignored.");
        (Some(value), None)
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
      (Some(Argument::Identifier(identifier)), None) => {
        let value = self.find_identifier_value(identifier);
        debug!("identifier value: {value:?}");
        let value_as_str = value.as_ref().and_then(|v| v.as_str());
        if let Some(value) = value_as_str {
          (Some(value.to_string()), None)
        } else {
          error!("Unable to parse {value:?}");
          todo!("Handle {value:?}");
        }
      },
      (None, None) => (None, None),
      (arg_1, arg_2) => {
        warn!("Unknown argument combination type: {arg_1:?} {arg_2:?}");
        todo!("Handle argument {arg_1:?} {arg_2:?}")
      },
    }
  }

  /// Get the namespace for an entry
  ///
  /// # Arguments
  ///
  /// * `key` - The key to get the namespace for
  /// * `options` - The options to get the namespace from
  pub(super) fn get_namespace(&self, options: Option<&I18NextOptions>, key: &str) -> (String, Option<String>) {
    let separator = self.options.namespace_separator.as_deref().unwrap_or(":");
    trace!("Namespace separator: {separator:?}", separator = separator.italic().cyan());
    let current_namespace = &self.current_namespace;
    trace!("Current namespace: {namespace:?}", namespace = current_namespace.italic().cyan());
    let ns_from_options = options.and_then(|o| o.get("namespace").cloned().flatten());
    trace!("Namespace from options: {namespace:?}", namespace = ns_from_options.italic().cyan());

    let (key, ns_from_key) = if key.contains(separator) {
      let mut split = key.split(separator);
      let ns = split.next().map(|v| v.to_string());
      let key = split.next().map(|v| v.to_string()).unwrap();
      (key, ns)
    } else {
      (key.to_string(), None)
    };
    trace!("Namespace from key: {namespace:?}", namespace = ns_from_key.italic().cyan());

    let namespace = ns_from_key.or(ns_from_options).or(current_namespace.clone());
    trace!("Namespace: {namespace:?}", namespace = namespace.italic().cyan());
    (key, namespace)
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

  pub(super) fn extract_jsx_entries(&mut self, elem: &JSXElement<'a>) {
    let key = self.get_prop_value(elem, "i18nKey");
    let ns = self.get_prop_value(elem, "ns");
    let default_value = self.get_prop_value(elem, "defaults");
    let count = self.has_prop(elem, "count");
    let context = self.get_prop_values(elem, "context");
    let options = self.get_prop_value(elem, "i18n");

    trace!("Childrens: {:?}", elem.children);
    let node_as_string = {
      let content = Self::parse_children(&elem.children);
      self.elem_to_string(&content)
    };
    trace!("Element as string: {node_as_string:?}");
    let default_value = default_value.unwrap_or(node_as_string);

    if let Some(key) = key {
      self.entries.push(Entry {
        location: Location::new(
          self.file_path.to_str().unwrap().to_string(),
          usize::try_from(elem.span.start).unwrap(),
          usize::try_from(elem.span.end).unwrap(),
        ),
        key,
        value: if default_value.is_empty() { None } else { Some(default_value) },
        namespace: ns,
        has_count: count,
        i18next_options: options.and_then(|v| serde_json::from_str(&v).ok()),
        context,
      });
    }
  }

  /// Extracts the key from the `t` function call.
  pub(super) fn extract_t_function_key(&mut self, expr: &CallExpression<'a>) -> Option<String> {
    match expr.arguments.first() {
      Some(Argument::StringLiteral(str)) => {
        trace!("t Arg: {:?}", str.bright_black().italic());
        Some(str.value.to_string().clone())
      },
      Some(Argument::TemplateLiteral(template)) => {
        trace!("t Arg: {:?}", template.bright_black().italic());
        trace!("t quasis: {:?}", template.quasis);
        trace!("t expressions: {:?}", template.expressions);
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &template.span);
          todo!("Handle template literal")
        }
        #[cfg(not(debug_assertions))]
        {
          warn!("Template literal are not supported for now");
          None
        }
      },
      Some(Argument::BinaryExpression(bin)) => {
        trace!("t Arg: {:?}", bin.bright_black().italic());
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &bin.span);
          todo!("Handle binary expression")
        }
        #[cfg(not(debug_assertions))]
        {
          warn!("Binary expression are not supported for now");
          None
        }
      },
      Some(Argument::CallExpression(expression)) => {
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &expression.span());
        }
        trace!("Skipping CallExpression as it is unsupported");
        None
      },
      Some(Argument::StaticMemberExpression(expression)) => {
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &expression.span());
        }
        trace!("Skipping StaticMemberExpression as it is unsupported");
        None
      },
      Some(Argument::Identifier(identifier)) => {
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &identifier.span());
        }
        trace!("Skipping Identifier as it is unsupported");
        None
      },
      Some(Argument::TSAsExpression(expression)) => {
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          let _ = print_error_location(&self.file_path, &expression.span());
        }
        trace!("Skipping TSAsExpression as it is unsupported");
        None
      },
      Some(arg) => {
        #[cfg(debug_assertions)]
        {
          use crate::visitor::visit::print_error_location;
          log::warn!("Unknown argument type found in [{}]: {arg:?}", self.file_path.display().yellow());
          let _ = print_error_location(&self.file_path, &arg.span());

          todo!("Handle argument {arg:?} in {}", self.file_path.display().yellow())
        }
        #[cfg(not(debug_assertions))]
        {
          warn!("Unknown argument type {arg:?}");
          None
        }
      },
      None => {
        warn!("No key provided, skipping entry");
        None
      },
    }
  }
}

fn find_type_of_identifier<'a>(
  identifier: &oxc_allocator::Box<'a, IdentifierReference<'a>>,
  param: &'a BindingPattern<'a>,
) -> Option<&'a oxc_allocator::Box<'a, ast::TSTypeAnnotation<'a>>> {
  debug!("Kind: {:#?}", param.kind);
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
                    ast::TSSignature::TSPropertySignature(signature)
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
  union_type: &oxc_allocator::Box<'_, ast::TSUnionType<'_>>,
) -> Option<Vec<String>> {
  if !union_type.types.iter().all(|t| matches!(t, oxc_ast::ast::TSType::TSLiteralType(_))) {
    warn!("Union type is not a TSTypeLiteral: {union_type:?}");
    return None::<Vec<String>>;
  }

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

  debug!("Found union type: {found:?}");

  Some(found)
}

#[cfg(test)]
mod tests {
  use oxc_allocator::Allocator;
  use oxc_ast_visit::Visit;
  use oxc_parser::Parser;
  use oxc_span::SourceType;

  use super::*;

  fn parse(source_text: &str) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();
    log::debug!("Program: {:#?}", ret.program.body);

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&program, "file.tsx", Config::default());
    visitor.visit_program(&program);
    visitor.entries
  }

  fn parse_with_options(source_text: &str) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&program, "file.tsx", Config::default());
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
    fn should_parse_t_with_namespace_from_name_first_with_t() {
      // language=javascript
      let source_text =
        "const t = useTranslation('other_override'); const title = t('namespace:toast.title', {ns: 'override'});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
    }

    #[test_log::test]
    fn should_parse_t_with_namespace_from_name_first() {
      // language=javascript
      let source_text = "const title = t('namespace:toast.title', {ns: 'override'});";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
    }

    #[test_log::test]
    fn should_parse_t_with_namespace_from_name() {
      // language=javascript
      let source_text = "const title = t('namespace:toast.title');";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
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

    #[test_log::test]
    fn should_parse_t_with_default_value_and_namespace_2() {
      // language=javascript
      let source_text =
        "const title = t('preview.error.text', 'An error has occurred while generating the preview.\\nPlease try again.', { ns: 'invoice', })";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new(
        "preview.error.text",
        "An error has occurred while generating the preview.\nPlease try again.",
        "invoice"
      )]);
    }

    #[test_log::test]
    fn should_parse_t_with_default_value_and_namespace_3() {
      let source_text = "context.showToast({
      title: t('preview.error.title', 'Error', { ns: 'invoice' }),
      text: t('preview.error.text', 'An error has occurred while generating the preview.\\nPlease try again.', {
        ns: 'invoice',
      }),
      variant: 'destructive',
      iconType: 'invoice',
    });";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 2);
      assert_eq!(keys, vec![
        Entry::new("preview.error.title", "Error", "invoice"),
        Entry::new(
          "preview.error.text",
          "An error has occurred while generating the preview.\nPlease try again.",
          "invoice"
        ),
      ]);
    }

    #[test_log::test]
    #[should_panic]
    fn should_parse_t_with_ns_defined_as_template_string() {
      // language=javascript
      let source_text = "const ns = 'ns'; const title = t(`${ns}:toast.title`, undefined);";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
    }

    #[test_log::test]
    fn should_parse_t_with_ns_defined_in_clone_instance() {
      // language=javascript
      let source_text = "const ns = 'ns'; const { t } = i18next.cloneInstance({ ns }); const title = t('toast.title');";
      let keys = parse(source_text);

      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
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
    fn should_extract_default_value_from_string_litteral_prop() {
      // language=javascript
      let source_text = "<Trans i18nKey='first' defaults='test-value'>should be ignored</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys, vec![Entry::new_with_value("first", "test-value")]);
    }

    #[test_log::test]
    fn should_extract_default_value_from_interpolated_string_prop() {
      // language=javascript
      let source_text = "<Trans i18nKey='first' defaults={'test-value'}>should be ignored</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys, vec![Entry::new_with_value("first", "test-value")]);
    }

    #[test_log::test]
    fn should_extract_key_from_self_closing() {
      // language=javascript
      let source_text = "<Trans i18nKey='first' />";
      let keys = parse(source_text);
      assert_eq!(keys, vec![Entry::empty("first")]);
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_format_interpolations_correctly() {
      // language=javascript
      let source_text = "<Trans count={count}>{{ key: property, format: 'number' }}</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys, vec![Entry::new_with_value("{{key, number}}", "{{key, number}}")]);
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_strip_invalid_interpolations() {
      // language=javascript
      let source_text = "<Trans count={count}>before{{ key1, key2 }}after</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys, vec![Entry::new_with_value("beforeafter", "beforeafter")]);
    }

    #[test_log::test]
    fn should_not_add_empty_for_self_closing_tags() {
      // language=javascript
      let source_text = "<Trans count={count}/>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      assert_eq!(keys, vec![]);
    }

    #[test_log::test]
    fn should_not_add_empty_for_empty_tags() {
      // language=javascript
      let source_text = "<Trans count={count}></Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      assert_eq!(keys, vec![]);
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_erases_tags_from_content() {
      // language=javascript
      let source_text = "<Trans>a<b test={'</b>'}>c<c>z</c></b>{d}<br stuff={y}/></Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      let first = keys.first().unwrap();
      assert_eq!(first.value, Some("a<1>c<1>z</1></1>{d}<3></3>".into()));
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_skips_dynamic_children() {
      // language=javascript
      let source_text =
        "<Trans>My dogs are named: <ul i18nIsDynamicList>{['rupert', 'max'].map(dog => (<li>{dog}</li>))}</ul></Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      let first = keys.first().unwrap();
      assert_eq!(first.value, Some("My dogs are named: <1></1>".into()));
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_handle_spread_attributes() {
      // language=javascript
      let source_text = "<Trans>My dog is named: <span {...styles}>Spot</span></Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      let first = keys.first().unwrap();
      assert_eq!(first.value, Some("My dog is named: <1>Spot</1>".into()));
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_erases_comment_expressions() {
      // language=javascript
      let source_text = "<Trans>{/* some comment */}Some Content</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      let first = keys.first().unwrap();
      assert_eq!(first.value, Some("Some Content".into()));
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_handles_jsx_fragments() {
      // language=javascript
      let source_text = "<><Trans i18nKey='first' /></>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      assert_eq!(keys, vec![Entry::empty("first")]);
    }

    #[test_log::test]
    #[should_panic] // todo: fix this test
    fn should_interpolates_literal_string_values() {
      // language=javascript
      let source_text = "<Trans>Some{' '}Interpolated {'Content'}</Trans>";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 0);
      let first = keys.first().unwrap();
      assert_eq!(first.value, Some("Some Interpolated Content".into()));
    }

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

    #[test_log::test]
    fn should_parse_jsx_context_from_string_arg_type_alias() {
      // language=javascript
      let source_text = "type Ctx = 'male' | 'female'; function El(val: Ctx) {return <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;}";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string(), "female".to_string())));
    }

    #[test_log::test]
    fn should_parse_jsx_context_from_props_arg_function() {
      // language=javascript
      let source_text = "function El({ val }: {val: 'male' | 'female'}) {return <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;}";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string(), "female".to_string())));
    }

    #[test_log::test]
    fn should_parse_jsx_context_from_string_arg_function() {
      // language=javascript
      let source_text = "function El(val: 'male' | 'female') {return <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;}";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string(), "female".to_string())));
    }

    #[test_log::test(ignore = "must be fixed")]
    fn should_parse_jsx_context_from_string_arg_const_function() {
      // language=javascript
      let source_text = "const El = (val: 'male' | 'female') => <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string())));
    }

    #[test_log::test]
    fn should_parse_jsx_context_from_variable_type() {
      // language=javascript
      let source_text =
        "const getSex = () => 'male'; const val: 'male' | 'female' = getSex(); const el = <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string(), "female".to_string())));
    }

    #[test_log::test]
    fn test_1() {
      // language=javascript
      let source_text = "function ThemeDropdownMenu() {
  const { t } = useTranslation('ns');
  const [theme, setTheme] = useTheme();
  const preferredTheme: 'dark' | 'light' = getPreferredTheme();

  return (
    <Trans context={preferredTheme} i18nKey='theme.system' ns='ns' t={t}>
        System theme
    </Trans>
  );
}";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("theme.system", "System theme", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("dark".to_string(), "light".to_string())));
    }

    #[test_log::test]
    fn should_parse_jsx_context_from_variable() {
      // language=javascript
      let source_text =
        "const val = 'male'; const el = <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string())));
    }

    #[test_log::test]
    fn should_parse_jsx_context_from_string_literal() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title' context='male'>Reset password</Trans>;";
      let keys = parse(source_text);
      assert_eq!(keys.len(), 1);
      assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      assert_eq!(keys.first().unwrap().context, Some(vec!("male".to_string())));
    }
  }
}
