use serde::Deserialize;
use serde_json::Value;
use std::{cmp::Ordering, collections::HashMap, fmt::Display, path::PathBuf};

#[derive(Clone, Default)]
pub struct Options {
  pub full_key_prefix: String,
  pub reset_and_flag: bool,
  pub keep_removed: Option<KeepRemoved>,
  pub key_separator: Option<String>,
  pub plural_separator: Option<String>,
  pub locale: String,
  pub suffix: Option<String>,
  pub separator: Option<String>,
  pub custom_value_template: Option<Value>,
  pub reset_default_value_locale: Option<String>,
  pub line_ending: String,
  pub create_old_catalogs: bool,
}

pub enum DefaultValue {
  Str(String),
  Func(Box<dyn Fn(Option<String>, Option<String>, Option<String>, Option<String>) -> String>),
}

#[derive(Clone, Debug)]
pub enum KeepRemoved {
  Bool(bool),
  Patterns(Vec<regex::Regex>),
}

impl Default for KeepRemoved {
  fn default() -> Self {
    KeepRemoved::Bool(false)
  }
}

#[derive(Clone, Debug, Default)]
pub enum KeySeparator {
  #[default]
  False,
  Str(String),
}

#[derive(Clone, Debug, Default)]
pub enum LineEnding {
  #[default]
  Auto,
  Crlf,
  Cr,
  Lf,
}

pub enum Sort {
  Bool(bool),
  Func(Box<dyn Fn(String, String) -> Ordering>),
}

pub struct UserConfig {
  context_separator: Option<String>,
  create_old_catalogs: Option<bool>,
  default_namespace: Option<String>,
  default_value: Option<DefaultValue>,
  indentation: Option<u32>,
  keep_removed: Option<KeepRemoved>,
  key_separator: Option<KeySeparator>,
  line_ending: Option<LineEnding>,
  locales: Option<Vec<String>>,
  namespace_separator: Option<KeySeparator>,
  output: Option<String>,
  plural_separator: Option<String>,
  input: Option<Vec<String>>,
  sort: Option<Sort>,
  verbose: Option<bool>,
  fail_on_warnings: Option<bool>,
  fail_on_update: Option<bool>,
  custom_value_template: Option<HashMap<String, String>>,
  reset_default_value_locale: Option<String>,
  i18next_options: Option<HashMap<String, serde_json::Value>>,
  yaml_options: Option<HashMap<String, serde_json::Value>>,
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
      log::error!("No configuration file found. Application may not behave as expected");
    }

    let cfg: Self = builder.build()?.try_deserialize()?;

    Ok(cfg)
  }
}
