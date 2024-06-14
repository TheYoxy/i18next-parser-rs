use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum LineEnding {
  #[default]
  Auto,
  Crlf,
  Cr,
  Lf,
}

impl From<LineEnding> for config::Value {
  #[inline]
  fn from(val: LineEnding) -> Self {
    match val {
      LineEnding::Auto => "auto".into(),
      LineEnding::Crlf => "crlf".into(),
      LineEnding::Cr => "cr".into(),
      LineEnding::Lf => "lf".into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_line_ending_auto() {
    let line_ending = LineEnding::Auto;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "auto".into());
  }

  #[test]
  fn test_line_ending_crlf() {
    let line_ending = LineEnding::Crlf;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "crlf".into());
  }

  #[test]
  fn test_line_ending_cr() {
    let line_ending = LineEnding::Cr;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "cr".into());
  }

  #[test]
  fn test_line_ending_lf() {
    let line_ending = LineEnding::Lf;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "lf".into());
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
  pub locales: Vec<String>,
  pub input: Vec<String>,
  pub output: String,
  pub context_separator: Option<String>,
  pub create_old_catalogs: bool,
  pub default_namespace: String,
  pub default_value: Option<String>,
  pub keep_removed: bool,
  pub key_separator: Option<String>,
  pub line_ending: LineEnding,
  pub namespace_separator: Option<String>,
  pub plural_separator: Option<String>,
  pub sort: bool,
  pub verbose: bool,
  pub fail_on_warnings: Option<bool>,
  pub fail_on_update: Option<bool>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
}

impl Default for Config {
  #[inline]
  fn default() -> Self {
    Self {
      locales: vec!["en".to_string()],
      output: "locales/$LOCALE/$NAMESPACE.json".to_string(),
      input: vec!["src/**/*.{ts,tsx}".to_string()],
      context_separator: Some("_".to_string()),
      default_namespace: "translation".to_string(),
      default_value: Some("".to_string()),
      keep_removed: false,
      key_separator: Some(".".to_string()),
      line_ending: LineEnding::Auto,
      namespace_separator: Some(":".to_string()),
      plural_separator: Some("_".to_string()),
      sort: true,
      verbose: false,
      create_old_catalogs: false,
      fail_on_warnings: None,
      fail_on_update: None,
      custom_value_template: None,
      reset_default_value_locale: None,
    }
  }
}

impl Config {
  pub fn new<T>(working_dir: T, verbose: bool) -> Result<Self, config::ConfigError>
  where
    T: Into<PathBuf>,
  {
    let default_config = Config::default();
    let mut builder = config::Config::builder()
      .set_default("locales", default_config.locales)?
      .set_default("output", default_config.output)?
      .set_default("input", default_config.input)?
      .set_default("context_separator", default_config.context_separator)?
      .set_default("default_namespace", default_config.default_namespace)?
      .set_default("default_value", default_config.default_value)?
      .set_default("keep_removed", default_config.keep_removed)?
      .set_default("key_separator", default_config.key_separator)?
      .set_default("line_ending", default_config.line_ending)?
      .set_default("namespace_separator", default_config.namespace_separator)?
      .set_default("plural_separator", default_config.plural_separator)?
      .set_default("create_old_catalogs", default_config.create_old_catalogs)?
      .set_default("sort", default_config.sort)?
      .set_default("verbose", default_config.verbose)?;

    if verbose {
      builder = builder.set_override("verbose", true)?;
    }

    let working_dir: PathBuf = working_dir.into();

    let config_files = [
      (".i18next-parser.json5", config::FileFormat::Json5),
      (".i18next-parser.json", config::FileFormat::Json),
      (".i18next-parser.yaml", config::FileFormat::Yaml),
      (".i18next-parser.toml", config::FileFormat::Toml),
      (".i18next-parser.ini", config::FileFormat::Ini),
      ("i18next-parser.json5", config::FileFormat::Json5),
      ("i18next-parser.json", config::FileFormat::Json),
      ("i18next-parser.yaml", config::FileFormat::Yaml),
      ("i18next-parser.toml", config::FileFormat::Toml),
      ("i18next-parser.ini", config::FileFormat::Ini),
    ];

    let mut found_config = false;
    for (file, format) in &config_files {
      builder = builder.add_source(config::File::from(working_dir.join(file)).format(*format).required(false));
      if working_dir.join(file).exists() {
        found_config = true
      }
    }

    if !found_config {
      log::error!("No configuration file found. Using default configuration.");
    }

    let mut cfg: Self = builder.build()?.try_deserialize()?;
    cfg.output = working_dir.join(&cfg.output).to_str().unwrap().to_string();
    Ok(cfg)
  }
}
