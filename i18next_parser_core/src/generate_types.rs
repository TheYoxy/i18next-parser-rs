//! This module is responsible for generating types for the i18next resources.
use std::{fmt::Display, fs, path::MAIN_SEPARATOR_STR};

use log::{info, trace};

use crate::{config::Config, merger::merge_results::MergeResults};

/// Converts a string to camel case.
fn camelize(s: &str) -> String {
  s.chars()
    .fold((String::new(), true, true), |(acc, is_first, at_separator), c| {
      if !c.is_alphanumeric() {
        (acc, is_first, true)
      } else if is_first {
        (acc + &c.to_ascii_lowercase().to_string(), false, false)
      } else if at_separator {
        (acc + &c.to_ascii_uppercase().to_string(), false, false)
      } else {
        (acc + &c.to_ascii_lowercase().to_string(), false, false)
      }
    })
    .0
}

/// Represents the value of an entry in the generated types.
#[derive(Debug)]
struct EntryValue<T: Display, P: Display, O: Display> {
  /// The visual of the entry.
  display_name: P,
  /// The name of the entry.
  name: O,
  /// The path of the entry.
  path: T,
}

/// Generates types for the i18next resources.
pub fn generate_types<C: AsRef<Config>>(entries: &[MergeResults], config: C) -> color_eyre::Result<()> {
  let config = config.as_ref();
  trace!("Generating types for i18next resources.");
  let default_locale = config
    .locales
    .first()
    .map_or("".to_string(), |p| format!("{}{}{}", MAIN_SEPARATOR_STR, p.as_str(), MAIN_SEPARATOR_STR));
  let result = entries
    .iter()
    .filter(|entry| {
      entry
        .path
        .strip_prefix(&config.working_dir)
        .is_ok_and(|s| s.to_str().map_or(false, |p| p.contains(default_locale.as_str())))
    })
    .map(|entry| {
      EntryValue {
        name: entry.namespace.as_str(),
        display_name: camelize(entry.namespace.as_str()),
        path: entry
          .path
          .strip_prefix(&config.working_dir)
          .unwrap_or_else(|_| panic!("Failed to strip prefix"))
          .to_str()
          .unwrap(),
      }
    })
    .collect::<Vec<_>>();

  let get_name_property = |name: &str| {
    if name.chars().any(|char| !char.is_alphanumeric()) {
      format!("'{}'", name)
    } else {
      name.to_string()
    }
  };

  let ns_separator = &config.namespace_separator;
  let key_separator = &config.key_separator;
  let context_separator = &config.context_separator;

  let imports = result
    .iter()
    .map(|entry| format!("import type {} from '{}';", entry.display_name, entry.path))
    .collect::<Vec<String>>()
    .join("\n");
  let resources = result
    .iter()
    .map(|entry| format!("{}: typeof {};", get_name_property(entry.name), entry.display_name))
    .collect::<Vec<String>>()
    .join("\n      ");
  let types = result.iter().map(|entry| format!("'{}'", entry.name)).collect::<Vec<String>>().join(" | ");
  let default_namespace = &config.default_namespace;
  let template = format!(
    r#"
// This file is generated automatically
// All changes will be lost
import 'i18next';

{imports}

declare module 'i18next' {{
  interface CustomTypeOptions {{
    defaultNS: '{default_namespace}';
    returnNull: false;
    returnObjects: false;
    nsSeparator: '{ns_separator}';
    keySeparator: '{key_separator}';
    contextSeparator: '{context_separator}';
    jsonFormat: 'v4';
    allowObjectInHTMLChildren: false;
    resources: {{
      {resources}
    }};
  }}
}}

declare global {{
  type Ns = {types};
}}
"#,
  );

  let generated_file_name = "react-i18next.resources.d.ts";
  let path = &config.working_dir.join(generated_file_name);
  fs::write(path, template)?;
  info!("Generated {}", generated_file_name);

  Ok(())
}

#[cfg(test)]
mod tests {
  use tempdir::TempDir;

  use super::*;
  use crate::merger::merge_results::MergeResults;

  #[test_log::test]
  fn camelize_transforms_strings_correctly() {
    assert_eq!(camelize("hello_world"), "helloWorld");
    assert_eq!(camelize("Hello-World"), "helloWorld");
    assert_eq!(camelize("testString"), "testString");
  }

  #[test_log::test]
  fn camelize_handles_empty_string() {
    assert_eq!(camelize(""), "");
  }

  #[test_log::test]
  fn camelize_handles_single_character() {
    assert_eq!(camelize("a"), "a");
    assert_eq!(camelize("A"), "a");
  }

  #[test_log::test]
  fn generate_types_creates_expected_output() {
    let temp = TempDir::new("generate_types").unwrap();
    let config = Config {
      working_dir: temp.path().to_path_buf(),
      locales: vec!["en".to_string()],
      namespace_separator: ':'.into(),
      key_separator: '.'.into(),
      context_separator: '_'.into(),
      default_namespace: "default".to_string(),
      ..Default::default()
    };

    let entries = vec![
      MergeResults {
        namespace: "namespace".to_string(),
        path: temp.path().join("en/namespace.json"),
        locale: "en".to_string(),
        merged: Default::default(),
        backup: Default::default(),
        old_catalog: Default::default(),
      },
      MergeResults {
        namespace: "another_namespace".to_string(),
        path: temp.path().join("en/another_namespace.json"),
        locale: "en".to_string(),
        merged: Default::default(),
        backup: Default::default(),
        old_catalog: Default::default(),
      },
    ];

    let generation = generate_types(&entries, &config);
    assert!(generation.is_ok());

    // Check that the generated file exists and contains the expected content.
    let result = fs::read_to_string(config.working_dir.join("react-i18next.resources.d.ts"));
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_ne!(content.len(), 0, "some content must be generated");
  }
}
