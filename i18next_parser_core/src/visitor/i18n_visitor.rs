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
use oxc_resolver::{ResolveOptions, Resolver, TsconfigOptions, TsconfigReferences};
use oxc_span::GetSpan;
use tracing::span;

use crate::{
  visitor::{
    node_child::NodeChild,
    traits::{oxc_custom_parser::OxcCustomParser, oxc_program::OxcProgram, print_error_location::PrintErrorLocation},
  },
  Config,
  Entry,
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
pub struct I18NVisitor<'a> {
  /// the program to be parsed
  pub program: &'a Program<'a>,
  allocator: &'a Allocator,
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
}
impl OxcCustomParser for I18NVisitor<'_> {
}

/// The visitor implementation that will search for translations inside javascript code
impl<'a> I18NVisitor<'a> {
  /// Creates a new \[`CountASTNodes`\].
  pub fn new<Path: Into<PathBuf>, C: AsRef<Config>>(
    allocator: &'a Allocator,
    program: &'a Program<'a>,
    file_path: Path,
    config: &'a C,
  ) -> Self {
    I18NVisitor {
      allocator,
      program,
      file_path: file_path.into(),
      entries: Default::default(),
      options: VisitorOptions::new(config),
      current_namespace: Default::default(),
      resolver: Resolver::new(ResolveOptions {
        roots: vec![config.as_ref().working_dir.join("src")],
        extensions: vec![".ts".into(), ".tsx".into(), ".js".into(), ".jsx".into()],
        extension_alias: vec![
          (".js".to_string(), vec![".js".to_string(), ".ts".to_string()]),
          (".jsx".to_string(), vec![".jsx".to_string(), ".tsx".to_string()]),
        ],
        prefer_relative: true,
        tsconfig: {
          let tsconfig = config.as_ref().working_dir.join("tsconfig.json");
          if tsconfig.exists() {
            Some(TsconfigOptions { config_file: tsconfig, references: TsconfigReferences::Auto })
          } else {
            None
          }
        },
        ..Default::default()
      }),
    }
  }

  #[cfg(feature = "print_error_location")]
  #[tracing::instrument(skip(self), target = "instrument")]
  pub fn print_error_location(&self, span: &oxc_span::Span) {
    use bat::{
      line_range::{LineRange, LineRanges},
      Input,
      PrettyPrinter,
    };

    #[inline]
    fn get_line_bounds(text: &str, start_char_index: usize, end_char_index: usize) -> Option<(usize, usize)> {
      if start_char_index > end_char_index || end_char_index > text.len() {
        return None; // Invalid indices
      }

      let mut current_line = 0; // 0-indexed line numbers

      let mut start_line: Option<usize> = None;
      let mut end_line: Option<usize> = None;

      // Iterate through characters and find the line bounds
      for (idx, c) in text.char_indices() {
        if idx >= start_char_index && start_line.is_none() {
          start_line = Some(current_line);
        }

        if idx >= end_char_index && end_line.is_none() {
          end_line = Some(current_line);
          // If we found both, we can break early
          if start_line.is_some() && end_line.is_some() {
            break;
          }
        }

        if c == '\n' {
          current_line += 1;
        }

        // If we've already passed the end_char_index by a significant margin
        // and haven't found end_line, it implies the end_char_index is within
        // the last line if the string doesn't end with a newline.
        // This break is an optimization.
        if end_line.is_some() && idx > end_char_index + 10 {
          // +10 is arbitrary for some buffer
          break;
        }
      }

      // Handle cases where the end_char_index is at the very end of the string
      // and there's no trailing newline, or if it's past the last newline.
      if let Some(s_line) = start_line {
        if let Some(e_line) = end_line {
          Some((s_line, e_line))
        } else {
          // If end_line was not set, it means the end_char_index
          // is within the very last line of the string.
          Some((s_line, current_line))
        }
      } else {
        // This case should ideally not happen if start_char_index is valid,
        // but included for robustness.
        None
      }
    }

    let content = self.program.source_text;

    let start_pos = usize::try_from(span.start).unwrap();
    let end_pos = usize::try_from(span.end).unwrap();
    let bounds = get_line_bounds(content, start_pos, end_pos);
    let (start_line, end_line) = match bounds {
      Some((start, end)) => (start + 1, end + 1), // Convert to 1-indexed lines
      None => {
        error!("{} Invalid span: {span:?}", "[Print_error_location]".red().bold());
        return;
      },
    };

    const BOUND: usize = 3;
    let range = LineRange::new(start_line.saturating_sub(BOUND), end_line + BOUND);
    let input = Input::from_bytes(content.as_bytes());
    let _ = PrettyPrinter::new()
      .input(input)
      .language(if self.program.source_type.is_typescript() { "typescript" } else { "javascript" })
      .line_ranges(LineRanges::from(vec![range]))
      .header(false)
      .grid(true)
      .line_numbers(true)
      .highlight_range(start_line, end_line)
      .print();
  }

  #[cfg(not(feature = "print_error_location"))]
  pub fn print_error_location(&self, _span: &oxc_span::Span) {
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
          let identifier = self.find_identifier_value_as_string(&identifier.name);
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
                    return self.find_identifier_value_as_string(&ident.name);
                  },
                  PropertyKey::Identifier(ident) if ident.name == "ns" => {
                    return self.find_identifier_value_as_string(&ident.name);
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
                  #[cfg(test)]
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
                          self.find_identifier_value_as_vec_string(&identifier.name)
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
                  trace!("Value: {attribute_name} {value:?}");
                  match value {
                    JSXAttributeValue::StringLiteral(str) => Some(str.value.to_string()),
                    JSXAttributeValue::ExpressionContainer(e) => {
                      // todo this expression will contains the required identifier
                      match &e.expression {
                        JSXExpression::StringLiteral(str) => Some(str.value.to_string()),
                        JSXExpression::Identifier(identifier) => {
                          trace!("Looking for identifier value for prop");
                          self.find_identifier_value_as_string(&identifier.name)
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
        if value.is_none() {
          (default_value, Some(i18next_options))
        } else {
          todo!("Handle identifier {identifier:?}")
        }
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

  pub(super) fn extract_jsx_entries(&mut self, elem: &JSXElement<'a>) {
    let key = self.get_prop_value_as_str(elem, "i18nKey");
    let ns = self.get_prop_value_as_str(elem, "ns");
    let default_value = self.get_prop_value_as_str(elem, "defaults");
    let count = self.has_prop(elem, "count");
    let context = self.get_prop_values_of_el(elem, "context");
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
