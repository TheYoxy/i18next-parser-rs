use std::{collections::HashMap, fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum KeySeparator {
  #[default]
  False,
  Str(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum LineEnding {
  #[default]
  Auto,
  Crlf,
  Cr,
  Lf,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Options {
  pub full_key_prefix: String,
  pub reset_and_flag: bool,
  pub keep_removed: Option<bool>,
  pub key_separator: Option<KeySeparator>,
  pub plural_separator: Option<String>,
  pub locale: String,
  pub suffix: Option<String>,
  pub separator: Option<String>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
  pub line_ending: LineEnding,
  pub create_old_catalogs: bool,
  pub namespace_separator: KeySeparator,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConfig {
  pub context_separator: Option<String>,
  pub create_old_catalogs: Option<bool>,
  pub default_namespace: Option<String>,
  pub default_value: Option<String>,
  pub indentation: Option<u32>,
  pub keep_removed: Option<bool>,
  pub key_separator: Option<KeySeparator>,
  pub line_ending: Option<LineEnding>,
  pub locales: Option<Vec<String>>,
  pub namespace_separator: Option<KeySeparator>,
  pub output: Option<String>,
  pub plural_separator: Option<String>,
  pub input: Option<Vec<String>>,
  pub sort: Option<bool>,
  pub verbose: Option<bool>,
  pub fail_on_warnings: Option<bool>,
  pub fail_on_update: Option<bool>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
  pub i18next_options: Option<HashMap<String, Value>>,
  pub yaml_options: Option<HashMap<String, Value>>,
}

impl From<UserConfig> for Options {
  fn from(val: UserConfig) -> Self {
    Options {
      create_old_catalogs: val.create_old_catalogs.unwrap_or(true),
      custom_value_template: val.custom_value_template,
      full_key_prefix: "".to_string(),
      keep_removed: val.keep_removed,
      key_separator: val.key_separator,
      line_ending: val.line_ending.unwrap_or(LineEnding::Auto),
      locale: val.locales.unwrap_or(vec!["en".to_string()])[0].clone(),
      plural_separator: val.plural_separator,
      reset_and_flag: val.fail_on_update.unwrap_or(false),
      reset_default_value_locale: val.reset_default_value_locale,
      separator: val.context_separator,
      suffix: None,
      namespace_separator: val.namespace_separator.unwrap_or(KeySeparator::Str(":".to_string())),
    }
  }
}

impl Default for UserConfig {
  fn default() -> Self {
    Self {
      context_separator: Some("_".to_string()),
      create_old_catalogs: Some(true),
      default_namespace: Some("translation".to_string()),
      default_value: Some("".to_string()),
      indentation: Some(2),
      keep_removed: Some(false),
      key_separator: Some(KeySeparator::Str(".".to_string())),
      line_ending: Some(LineEnding::Auto),
      locales: Some(vec!["en".to_string()]),
      namespace_separator: Some(KeySeparator::Str(":".to_string())),
      output: Some("locales/$LOCALE/$NAMESPACE.json".to_string()),
      plural_separator: Some("_".to_string()),
      input: Some(vec!["src/**/*.{ts,tsx}".to_string()]),
      sort: Some(true),
      verbose: Some(false),
      fail_on_warnings: Some(false),
      fail_on_update: Some(false),
      custom_value_template: None,
      yaml_options: None,
      i18next_options: {
        let mut map = HashMap::<String, Value>::new();

        map.insert("nsSeparator".to_string(), Value::String(":".to_string()));
        map.insert("keySeparator".to_string(), Value::String(".".to_string()));
        map.insert("pluralSeparator".to_string(), Value::String("_".to_string()));

        Some(map)
      },
      reset_default_value_locale: None,
    }
  }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
  #[serde(default)]
  pub _data_dir: PathBuf,
  #[serde(default)]
  pub _config_dir: PathBuf,
  #[serde(default)]
  pub working_dir: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
  #[serde(default, flatten)]
  pub config: AppConfig,
  #[serde(default)]
  pub options: UserConfig,
}

impl Config {
  pub fn new<T>(working_dir: Option<T>) -> Result<Self, config::ConfigError>
  where
    T: Into<config::Value> + Display,
  {
    let data_dir = crate::utils::get_data_dir();
    let config_dir = crate::utils::get_config_dir();
    let mut builder = config::Config::builder()
      .set_default("_data_dir", data_dir.to_str().unwrap())?
      .set_default("_config_dir", config_dir.to_str().unwrap())?;

    if let Some(working_dir) = working_dir {
      log::debug!("overriding working_dir to {}", working_dir);
      builder = builder.set_override("working_dir", working_dir)?;
    }

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
      builder = builder.add_source(config::File::from(config_dir.join(file)).format(*format).required(false));
      if config_dir.join(file).exists() {
        found_config = true
      }
    }
    if !found_config {
      log::error!("No configuration file found. Using default configuration.");
    }

    let cfg: Self = builder.build()?.try_deserialize()?;

    Ok(cfg)
  }
}
