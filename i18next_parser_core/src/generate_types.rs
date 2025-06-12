//! This module is responsible for generating types for the i18next resources.
use std::{collections::HashMap, fmt::Display, fs, path::Path};

use color_eyre::owo_colors::OwoColorize;
use log::{info, trace};

use crate::{config::Config, merger::merge_results::MergeResults};

/// Converts a string to camel case.
#[inline]
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
struct EntryValue<T: Display, P: Display, O: Display, L: Display> {
  /// The visual of the entry.
  display_name: P,
  /// The locale of the entry.
  locale: L,
  /// The name of the entry.
  name: O,
  /// The path of the entry.
  path: T,
}

pub fn generate_index<C: AsRef<Config>>(entries: &[MergeResults], config: C) -> color_eyre::Result<()> {
  let config = config.as_ref();
  trace!("Generating index for i18next resources.");

  let result = entries
    .iter()
    .map(|entry| {
      EntryValue {
        name: entry.namespace.as_str(),
        display_name: format!("{}_{}", camelize(entry.namespace.as_str()), entry.locale),
        locale: entry.locale.as_str(),
        path: entry
          .path
          .strip_prefix(&config.working_dir)
          .unwrap_or_else(|_| panic!("Failed to strip prefix"))
          .to_str()
          .unwrap(),
      }
    })
    .collect::<Vec<_>>();

  for local in &config.locales {
    let exports = result
      .iter()
      .filter(|entry| entry.locale == local)
      .map(|entry| format!("export * from './{}.json';", entry.name))
      .collect::<Vec<String>>()
      .join("\n");
    let output = config.output.replace("$LOCALE", local).replace("/$NAMESPACE.json", "/index.ts");
    write_exports(config, &exports, &output)?;
  }

  let exports =
    config.locales.iter().map(|locale| format!("export * from './{locale}';")).collect::<Vec<String>>().join("\n");
  write_exports(config, &exports, &config.output.replace("/$LOCALE", "").replace("/$NAMESPACE.json", "/index.ts"))?;

  Ok(())
}

fn write_exports(config: &Config, exports: &String, output: &String) -> Result<(), color_eyre::eyre::Error> {
  let template = format!(
    r#"
/// This file is generated automatically
/// All changes will be lost
/* eslint-disable */

{exports}

export default {{}};
"#,
  );
  let display = config.working_dir.display();
  let path = format!("{display}/{output}");
  let path = Path::new(&path);
  if !config.dry_run {
    log::debug!("Writing {}", path.display().yellow().italic());
    fs::write(path, template)?;
    info!("Generated {}", path.display());
  }

  Ok(())
}

/// Generates types for the i18next resources.
pub fn generate_types<C: AsRef<Config>>(entries: &[MergeResults], config: C) -> color_eyre::Result<()> {
  let config = config.as_ref();
  trace!("Generating types for i18next resources.");
  let result = entries
    .iter()
    .map(|entry| {
      EntryValue {
        name: entry.namespace.as_str(),
        display_name: format!("{}_{}", camelize(entry.namespace.as_str()), entry.locale),
        locale: entry.locale.as_str(),
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
    if name.chars().any(|char| !char.is_alphanumeric()) { format!("'{name}'") } else { name.to_string() }
  };

  let ns_separator = &config.namespace_separator;
  let key_separator = &config.key_separator;
  let context_separator = &config.context_separator;

  let imports = result
    .iter()
    .map(|entry| format!("import type {} from '{}';", entry.display_name, entry.path))
    .collect::<Vec<String>>()
    .join("\n");

  let has_default_namespace = result.binary_search_by(|entry| entry.name.cmp(config.default_namespace.as_str()));
  let custom_resources = if has_default_namespace.is_err() {
    let mut vec = result
      .iter()
      .map(|entry| format!("{}: typeof {};", get_name_property(entry.name), entry.display_name))
      .collect::<Vec<String>>();
    vec.push(format!("{}: {{}}", get_name_property(config.default_namespace.as_str())));
    vec.join("\n      ")
  } else {
    result
      .iter()
      .map(|entry| format!("{}: typeof {};", get_name_property(entry.name), entry.display_name))
      .collect::<Vec<String>>()
      .join("\n      ")
  };

  let mut resource_map = HashMap::new();
  for entry in result.iter() {
    if !resource_map.contains_key(entry.locale) {
      resource_map.insert(entry.locale, vec![]);
    }
    let map_entry = resource_map.get_mut(entry.locale).unwrap();
    map_entry.push(format!("{}: typeof {};", get_name_property(entry.name), entry.display_name));
  }

  let mut resources = String::new();
  for (key, value) in resource_map {
    resources += &format!("{}: {{\n{}\n}},\n", key, value.join("\n        "));
  }

  let mut types = result.iter().map(|entry| format!("'{}'", entry.name)).collect::<Vec<String>>();

  types.sort();
  types.dedup();

  let types = types.join(" | ");
  let default_namespace = &config.default_namespace;

  let template = format!(
    r#"
/// This file is generated automatically
/// All changes will be lost
/* eslint-disable */

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
      {custom_resources}
    }};
  }}

  interface Resource {{
    {resources}
  }}
}}

declare global {{
  type Ns = {types};
}}

export default {{}};
"#,
  );

  if config.dry_run {
    return Ok(());
  }
  let path = Path::new(&config.generated_types);
  log::debug!("Writing {}", path.display().yellow().italic());
  fs::write(path, template)?;
  info!("Generated {}", Path::new(path).display());

  Ok(())
}

#[cfg(test)]
mod tests {
  use tempdir::TempDir;

  use super::*;
  use crate::merger::merge_results::MergeResults;

  #[test_log::test]
  fn camelize_transforms_strings_to_lowercase_correctly() {
    assert_eq!(camelize("testString"), "teststring");
    assert_eq!(camelize("TESTSTRING"), "teststring");
    assert_eq!(camelize("T-ESTSTRING"), "tEststring");
  }

  #[test_log::test]
  fn camelize_transforms_strings_correctly() {
    assert_eq!(camelize("hello_world"), "helloWorld");
    assert_eq!(camelize("Hello-World"), "helloWorld");
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

  #[test_log::test(ignore = "this should be fixed")]
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
