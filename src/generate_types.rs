use std::fmt::Display;
use std::fs;
use std::path::MAIN_SEPARATOR_STR;

use regex::Regex;

use crate::config::Config;
use crate::file::MergeResults;
use crate::printinfo;

/// Converts a string to camel case.
fn camelize(s: &str) -> String {
  let re = Regex::new(r"^([A-Z])|[\s\-_](\w)").unwrap();

  re.replace_all(s, |caps: &regex::Captures| {
    if let Some(m) = caps.get(2) {
      m.as_str().to_uppercase()
    } else {
      caps.get(1).unwrap().as_str().to_lowercase()
    }
  })
  .to_string()
}

#[derive(Debug)]
struct EntryValue<T: Display, P: Display, O: Display> {
  display_name: P,
  name: O,
  path: T,
}

pub(crate) fn generate_types<C: AsRef<Config>>(entries: &[MergeResults], config: C) -> color_eyre::Result<()> {
  let config = config.as_ref();
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
    .map(|entry| EntryValue {
      name: entry.namespace.as_str(),
      display_name: camelize(entry.namespace.as_str()),
      path: entry
        .path
        .strip_prefix(&config.working_dir)
        .unwrap_or_else(|_| panic!("Failed to strip prefix"))
        .to_str()
        .unwrap(),
    })
    .collect::<Vec<_>>();

  let get_name_property = |name: &str| {
    if Regex::new(r"\W").unwrap().is_match(name) {
      format!("'{}'", name)
    } else {
      name.to_string()
    }
  };

  let ns_separator = &config.namespace_separator;
  let key_separator = &config.key_separator;
  let context_separator = &config.context_separator;

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
    imports = result
      .iter()
      .map(|entry| format!("import type {} from '{}';", entry.display_name, entry.path))
      .collect::<Vec<String>>()
      .join("\n"),
    resources = result
      .iter()
      .map(|entry| format!("{}: typeof {};", get_name_property(entry.name), entry.display_name))
      .collect::<Vec<String>>()
      .join("\n      "),
    types = result.iter().map(|entry| format!("'{}'", entry.name)).collect::<Vec<String>>().join(" | ")
  );

  let generated_file_name = "react-i18next.resources.d.ts";
  fs::write(config.working_dir.join(generated_file_name), template)?;
  printinfo!("Generated {}", generated_file_name);

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_camelize() {
    assert_eq!(camelize("hello_world"), "helloWorld");
    assert_eq!(camelize("Hello-World"), "helloWorld");
    assert_eq!(camelize("testString"), "testString");
  }
}
