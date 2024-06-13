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

impl From<config::ValueKind> for LineEnding {
  #[inline]
  fn from(kind: config::ValueKind) -> Self {
    if let config::ValueKind::String(s) = kind {
      match s.as_str() {
        "auto" => LineEnding::Auto,
        "crlf" => LineEnding::Crlf,
        "cr" => LineEnding::Cr,
        "lf" => LineEnding::Lf,
        _ => LineEnding::Auto,
      }
    } else {
      LineEnding::Auto
    }
  }
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

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Options {
  pub verbose: bool,
  pub full_key_prefix: String,
  pub reset_and_flag: bool,
  pub keep_removed: Option<bool>,
  pub key_separator: Option<String>,
  pub plural_separator: Option<String>,
  pub locales: Vec<String>,
  pub suffix: Option<String>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
  pub line_ending: LineEnding,
  pub create_old_catalogs: bool,
  pub namespace_separator: Option<String>,
  pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
  pub context_separator: Option<String>,
  pub create_old_catalogs: Option<bool>,
  pub default_namespace: Option<String>,
  pub default_value: Option<String>,
  pub keep_removed: Option<bool>,
  pub key_separator: Option<String>,
  pub line_ending: Option<LineEnding>,
  pub locales: Vec<String>,
  pub namespace_separator: Option<String>,
  pub output: String,
  pub plural_separator: Option<String>,
  pub input: Vec<String>,
  pub sort: Option<bool>,
  pub verbose: bool,
  pub fail_on_warnings: Option<bool>,
  pub fail_on_update: Option<bool>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
}

impl From<&Config> for Options {
  #[inline]
  fn from(val: &Config) -> Self {
    Options {
      create_old_catalogs: val.create_old_catalogs.unwrap_or(true),
      custom_value_template: val.custom_value_template.clone(),
      full_key_prefix: "".to_string(),
      keep_removed: val.keep_removed,
      key_separator: val.key_separator.clone(),
      line_ending: val.line_ending.clone().unwrap_or(LineEnding::Auto),
      locales: val.locales.clone(),
      plural_separator: val.plural_separator.clone(),
      reset_and_flag: val.fail_on_update.unwrap_or(false),
      reset_default_value_locale: val.reset_default_value_locale.clone(),
      suffix: None,
      namespace_separator: val.namespace_separator.clone(),
      verbose: val.verbose,
      output: val.output.clone(),
    }
  }
}

impl From<Config> for Options {
  #[inline]
  fn from(val: Config) -> Self {
    Options {
      create_old_catalogs: val.create_old_catalogs.unwrap_or(true),
      custom_value_template: val.custom_value_template,
      full_key_prefix: "".to_string(),
      keep_removed: val.keep_removed,
      key_separator: val.key_separator,
      line_ending: val.line_ending.unwrap_or(LineEnding::Auto),
      locales: val.locales.clone(),
      plural_separator: val.plural_separator,
      reset_and_flag: val.fail_on_update.unwrap_or(false),
      reset_default_value_locale: val.reset_default_value_locale,
      suffix: None,
      namespace_separator: val.namespace_separator.clone(),
      verbose: val.verbose,
      output: val.output.clone(),
    }
  }
}

impl Config {
  pub fn new<T>(working_dir: T, verbose: bool) -> Result<Self, config::ConfigError>
  where
    T: Into<PathBuf>,
  {
    let mut builder = config::Config::builder()
      .set_default("locales", vec!["en".to_string()])?
      .set_default("output", "locales/$LOCALE/$NAMESPACE.json")?
      .set_default("input", vec!["src/**/*.{ts,tsx}".to_string()])?
      .set_default("context_separator", "_")?
      .set_default("default_namespace", "translation")?
      .set_default("default_value", "")?
      .set_default("keep_removed", false)?
      .set_default("key_separator", ".")?
      .set_default("line_ending", LineEnding::Auto)?
      .set_default("namespace_separator", ":")?
      .set_default("plural_separator", "_")?
      .set_default("sort", true)?
      .set_default("verbose", false)?;

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
