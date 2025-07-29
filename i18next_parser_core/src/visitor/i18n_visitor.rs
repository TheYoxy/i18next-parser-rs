use std::{collections::HashMap, path::PathBuf};

use color_eyre::owo_colors::OwoColorize;
use log::{debug, error, trace, warn};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
  Argument,
  CallExpression,
  JSXAttributeItem,
  JSXAttributeName,
  JSXAttributeValue,
  JSXElement,
  JSXExpression,
  ObjectPropertyKind,
  Program,
  PropertyKey,
};
use oxc_resolver::Resolver;
use oxc_span::GetSpan;
use serde_json::Value;
use tracing::span;

use crate::{
  Config,
  Entry,
  Location,
  helper::{
    MakeRelativePath,
    SerdeHelper,
    html_entities_replacer::decode_html_entities,
    resolver_helper::ResolveFromTsConfig,
  },
  visitor::{
    node_child::NodeChild,
    traits::{
      GetLineBound,
      get_line_bounds,
      oxc_custom_parser::OxcCustomParser,
      oxc_program::OxcProgram,
      print_error_location::PrintErrorLocation,
    },
  },
};

/// This type alias represents the options for i18next.
/// It is a HashMap where the key is a String representing the option name,
/// and the value is an Option<`String`> representing the option value.
pub type I18NextOptions = HashMap<String, Option<Value>>;

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
pub struct I18NVisitor<'a> {
  /// the program to be parsed
  pub program: &'a Program<'a>,
  allocator: &'a Allocator,
  working_dir: &'a PathBuf,
  /// the file name of the file being parsed
  pub file_path: PathBuf,
  /// the entries in the i18n system
  pub entries: Vec<Entry>,
  /// the options for the I18NVisitor
  pub options: VisitorOptions,
  /// the current namespace while parsing a file
  pub(super) current_namespace: Option<String>,
  pub(super) resolver: Resolver,
}

impl PrintErrorLocation for I18NVisitor<'_> {
}
impl OxcProgram for I18NVisitor<'_> {
  fn program(&self) -> &Program<'_> {
    self.program
  }

  fn file_path(&self) -> &PathBuf {
    &self.file_path
  }

  fn resolver(&self) -> &Resolver {
    &self.resolver
  }

  fn allocator(&self) -> &Allocator {
    self.allocator
  }

  fn working_dir(&self) -> &PathBuf {
    self.working_dir
  }
}
impl GetLineBound for I18NVisitor<'_> {
}
impl OxcCustomParser for I18NVisitor<'_> {
}

/// The visitor implementation that will search for translations inside javascript code
impl<'a> I18NVisitor<'a> {
  /// Creates a new \[`CountASTNodes`\].
  pub fn new<Path: Into<PathBuf> + Clone, C: AsRef<Config>>(
    allocator: &'a Allocator,
    program: &'a Program<'a>,
    file_path: Path,
    config: &'a C,
  ) -> Self {
    let working_dir = &config.as_ref().working_dir;

    I18NVisitor {
      allocator,
      program,
      working_dir,
      file_path: file_path.clone().into(),
      entries: Default::default(),
      options: VisitorOptions::new(config),
      current_namespace: Default::default(),
      resolver: Resolver::from_ts_config(file_path.clone()).unwrap_or_default(),
    }
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
          trace!("Getting namespace from {} {}", name.cyan(), value.blue());
          self.current_namespace = if value.is_empty() { None } else { Some(value) };
        },
        Argument::Identifier(identifier) => {
          trace!("Looking for namespace {} value from identifier", name.cyan());
          let identifier = self.find_identifier_value_as_serde(&identifier.name);
          self.current_namespace = identifier.and_then(|i| i.as_str().map(|i| i.to_string()));
        },
        Argument::TSAsExpression(expression) => {
          trace!("Looking for namespace {} value from `As` expression", name.cyan());
          self.current_namespace =
            self.parse_expression_as_serde(&expression.expression).and_then(|i| i.as_str().map(|i| i.to_string()));
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
                    return self
                      .find_identifier_value_as_serde(&ident.name)
                      .and_then(|i| i.as_str().map(|i| i.to_string()));
                  },
                  PropertyKey::Identifier(ident) if ident.name == "ns" => {
                    return self
                      .find_identifier_value_as_serde(&ident.name)
                      .and_then(|i| i.as_str().map(|i| i.to_string()));
                  },
                  _ => (),
                }
                self.print_error_location(&prop.span());
                warn!("{} Unsupported property key: {:?}", "[Extract_namespace]".red().bold(), obj.key);
              }

              None
            })
            .collect::<Vec<String>>();
          let value = vec.first();
          self.current_namespace = value.cloned();
        },
        arg => {
          self.print_error_location(&arg.span());
          warn!("{} Unsupported argument for {name} {arg:?}", "[Extract_namespace]".red().bold(),);
        },
      }
    }
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
  pub(super) fn get_prop_values_of_el(&self, elem: &JSXElement<'_>, attribute_name: &str) -> Option<Vec<String>> {
    _ = span!(tracing::Level::TRACE, "get_prop_value", attribute_name = attribute_name).enter();
    let ret = elem
      .opening_element
      .attributes
      .iter()
      .filter_map(|elem| {
        match elem {
          JSXAttributeItem::Attribute(attribute) => {
            if let JSXAttributeName::Identifier(identifier) = &attribute.name {
              if identifier.name == attribute_name {
                if let Some(value) = &attribute.value {
                  #[cfg(debug_assertions)]
                  trace!(
                    "Value: {attribute_name} {value:?}",
                    attribute_name = attribute_name.cyan(),
                    value = value.bright_black().italic()
                  );
                  match value {
                    JSXAttributeValue::StringLiteral(str) => Some(vec![str.value.to_string()]),
                    JSXAttributeValue::ExpressionContainer(e) => {
                      // todo this expression will contains the required identifier
                      match &e.expression {
                        JSXExpression::StringLiteral(str) => Some(vec![str.value.to_string()]),
                        JSXExpression::Identifier(identifier) => {
                          self.find_identifier_value_as_serde(&identifier.name).value_to_string_vec()
                        },
                        JSXExpression::NumericLiteral(num) => Some(vec![num.value.to_string()]),
                        JSXExpression::StaticMemberExpression(expression) => {
                          self.parse_expression_as_serde(&expression.object).value_to_string_vec()
                        },
                        JSXExpression::TSAsExpression(expression) => {
                          self.parse_expression_as_serde(&expression.expression).value_to_string_vec()
                        },
                        _ => todo!("expression container {e:?} not supported in {}", self.file_path.display().yellow()),
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
      .next();

    if let Some(ret) = &ret {
      trace!("{} Found value: {ret:?}", "[get_prop_values_of_el]".blue());
    } else {
      trace!("{} {} found for expression", "[get_prop_values_of_el]".blue(), "No value".red().bold());
    }
    ret
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
  pub(super) fn get_prop_value_as_str(&self, elem: &JSXElement<'_>, attribute_name: &str) -> Option<String> {
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
                  trace!(
                    "Value: {attribute_name} {value:?}",
                    attribute_name = attribute_name.cyan(),
                    value = value.bright_black().italic()
                  );
                  match value {
                    JSXAttributeValue::StringLiteral(str) => Some(str.value.to_string()),
                    JSXAttributeValue::ExpressionContainer(e) => {
                      // todo this expression will contains the required identifier
                      match &e.expression {
                        JSXExpression::StringLiteral(str) => Some(str.value.to_string()),
                        JSXExpression::Identifier(identifier) => {
                          trace!("Looking for identifier value for prop");
                          self
                            .find_identifier_value_as_serde(&identifier.name)
                            .and_then(|i| i.as_str().map(|i| i.to_string()))
                        },
                        JSXExpression::NumericLiteral(num) => Some(num.value.to_string()),
                        JSXExpression::StaticMemberExpression(expression) => {
                          self
                            .parse_expression_as_serde(&expression.object)
                            .and_then(|i| i.as_str().map(|i| i.to_string()))
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
  pub(super) fn jsx_element_to_string(&self, childs: &[NodeChild]) -> String {
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
            let element_name = if use_tag_name { tag_name } else { &format!("{index}") };
            let children_string = tag.children.as_ref().map(|v| self.jsx_element_to_string(v)).unwrap_or_default();
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
      (Some(Argument::StringLiteral(str)), Some(Argument::Identifier(_))) => {
        let value = str.value.to_string();
        trace!("translation value defined as string literal: {}", value.cyan());
        #[cfg(debug_assertions)]
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
        let value = self.find_identifier_value_as_serde(&identifier.name);
        let (i18next_options, default_value) = self.parse_option_and_default_value(obj);
        if value.is_none() { (default_value, Some(i18next_options)) } else { todo!("Handle identifier {identifier:?}") }
      },
      (Some(Argument::Identifier(identifier)), None) => {
        let value = self.find_identifier_value_as_serde(&identifier.name);
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
    let ns_from_options =
      options.and_then(|o| o.get("namespace").cloned().flatten().and_then(|v| v.as_str().map(|s| s.to_string())));
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

  pub(super) fn extract_jsx_entries(&mut self, elem: &JSXElement<'a>) {
    let key = self.get_prop_value_as_str(elem, "i18nKey");
    let ns = self.get_prop_value_as_str(elem, "ns");
    let default_value = self.get_prop_value_as_str(elem, "defaults");
    let count = self.has_prop(elem, "count");
    let context = self.get_prop_values_of_el(elem, "context");

    if context.is_none()
      && let Some(val) = elem.opening_element.attributes.iter().find(|attr| {
        if let JSXAttributeItem::Attribute(attr) = attr {
          if let JSXAttributeName::Identifier(name) = &attr.name { name.name == "context" } else { false }
        } else {
          false
        }
      })
      && let Some(attribute) = val.as_attribute()
    {
      trace!("Print missing context");
      let val = match &attribute.value {
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
          container.expression.as_expression().and_then(|e| e.get_identifier_reference()).map(|id| id.name)
        },
        _ => None,
      };

      if let Some(val) = val {
        let line = get_line_bounds(
          self.program.source_text,
          usize::try_from(attribute.span.start).expect("span size overload"),
          usize::try_from(attribute.span.end).expect("span size overload"),
        )
        .map(|(start, end)| if start == end { format!(":{start}") } else { format!(":{start}-{end}") })
        .unwrap_or_default();
        warn!(
          "Unable to find the value of {key} {value:?} in {file_name}{line}",
          key = "context".cyan(),
          value = val.blue().bold(),
          file_name = self.file_path.make_relative(self.working_dir).display().yellow().dimmed(),
          line = line.blue()
        );
        self.print_error_location(&attribute.span);
      }
    };
    let options = self.get_prop_value_as_str(elem, "i18n");

    trace!("Childrens: {:?}", elem.children);
    let node_as_string = {
      let content = Self::parse_jsx_children(&elem.children);
      self.jsx_element_to_string(&content)
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
        value: if default_value.is_empty() { None } else { decode_html_entities(&default_value).ok() },
        namespace: ns,
        has_count: count,
        i18next_options: options.and_then(|v| serde_json::from_str(&v).ok()),
        context,
      });
    }
  }

  /// Extracts the key from the `t` function call.
  #[allow(unused_variables)]
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
          self.print_error_location(&template.span);
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
          self.print_error_location(&bin.span);
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
          self.print_error_location(&expression.span());
        }
        trace!("Skipping CallExpression as it is unsupported");
        None
      },
      Some(Argument::StaticMemberExpression(expression)) => {
        #[cfg(debug_assertions)]
        {
          self.print_error_location(&expression.span());
        }
        trace!("Skipping StaticMemberExpression as it is unsupported");
        None
      },
      Some(Argument::Identifier(identifier)) => {
        #[cfg(debug_assertions)]
        {
          self.print_error_location(&identifier.span());
        }
        trace!("Skipping Identifier as it is unsupported");
        None
      },
      Some(Argument::TSAsExpression(expression)) => {
        #[cfg(debug_assertions)]
        {
          self.print_error_location(&expression.span());
        }
        trace!("Skipping TSAsExpression as it is unsupported");
        None
      },
      Some(arg) => {
        #[cfg(debug_assertions)]
        {
          log::warn!("Unknown argument type found in [{}]: {arg:?}", self.file_path.display().yellow());
          self.print_error_location(&arg.span());

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
