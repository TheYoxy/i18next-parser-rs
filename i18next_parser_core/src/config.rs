//! This module provides configuration for the i18n system.
use std::path::{PathBuf, MAIN_SEPARATOR_STR};

use color_eyre::owo_colors::OwoColorize;
use config::{File, FileFormat, FileSourceFile};
use serde::{Deserialize, Serialize};

/// Line ending configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum LineEnding {
  /// Auto-detect line endings.
  #[default]
  Auto,
  /// Use CRLF line endings.
  Crlf,
  /// Use CR line endings.
  Cr,
  /// Use LF line endings.
  Lf,
}

/// Convert `LineEnding` to `config::Value`.
impl From<LineEnding> for config::Value {
  /// Convert `LineEnding` to `config::Value`.
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
/// This struct represents the configuration for the i18n system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
  /// The working directory for the i18n system.
  pub working_dir: PathBuf,
  /// A vector of locales used in the i18n system.
  pub locales: Vec<String>,
  /// A vector of input sources for the i18n system.
  pub input: Vec<String>,
  /// The output destination for the i18n system.
  pub output: String,
  /// The separator used in the context of the i18n system.
  pub context_separator: String,
  /// A boolean indicating whether to create old catalogs in the i18n system.
  pub create_old_catalogs: bool,
  /// The default namespace used in the i18n system.
  pub default_namespace: String,
  /// The default value used in the i18n system.
  pub default_value: String,
  /// A boolean indicating whether to keep removed entries in the i18n system.
  pub keep_removed: bool,
  /// The separator used for keys in the i18n system.
  pub key_separator: String,
  /// The line ending configuration for the i18n system.
  pub line_ending: LineEnding,
  /// The separator used for namespaces in the i18n system.
  pub namespace_separator: String,
  /// The separator used for plurals in the i18n system.
  pub plural_separator: String,
  /// A boolean indicating whether to sort entries in the i18n system.
  pub sort: bool,
  /// A boolean indicating whether to output verbose logs in the i18n system.
  pub verbose: bool,
  /// A boolean indicating whether to fail on warnings in the i18n system.
  pub fail_on_warnings: bool,
  /// A boolean indicating whether to fail on updates in the i18n system.
  pub fail_on_update: bool,
  /// An optional string representing the locale to reset the default value in the i18n system.
  pub reset_default_value_locale: Option<String>,
}

impl AsRef<Config> for Config {
  fn as_ref(&self) -> &Config {
    self
  }
}

impl Default for Config {
  #[inline]
  fn default() -> Self {
    Self {
      working_dir: PathBuf::from("."),
      locales: vec!["en".into()],
      output: ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR),
      input: vec!["src/**/*.{ts,tsx}".into()],
      context_separator: "_".into(),
      default_namespace: "translation".into(),
      default_value: "".into(),
      keep_removed: Default::default(),
      key_separator: ".".into(),
      line_ending: LineEnding::Auto,
      namespace_separator: ":".into(),
      plural_separator: "_".into(),
      sort: true,
      verbose: Default::default(),
      create_old_catalogs: Default::default(),
      fail_on_warnings: Default::default(),
      fail_on_update: Default::default(),
      reset_default_value_locale: Default::default(),
    }
  }
}

/// Implement `Config`.
impl Config {
  /// Create a new instance of `Config`.
  /// # Arguments
  /// * `working_dir` - The working directory for the i18n system.
  /// * `verbose` - A boolean indicating whether to output verbose logs in the i18n system.
  pub fn new<T>(working_dir: T, verbose: bool) -> Result<Self, config::ConfigError>
  where
    T: Into<PathBuf>,
  {
    let default_config = Config::default();
    let working_dir: PathBuf = working_dir.into();
    let working_dir_opt: &str = working_dir.as_path().to_str().unwrap();
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
      .set_default("verbose", default_config.verbose)?
      .set_default("fail_on_warnings", default_config.fail_on_warnings)?
      .set_default("fail_on_update", default_config.fail_on_update)?
      .set_override("working_dir", working_dir_opt)?;

    if verbose {
      builder = builder.set_override("verbose", true)?;
    }

    let config_files = [
      (".i18next-parser.json5", FileFormat::Json5),
      (".i18next-parser.json", FileFormat::Json),
      (".i18next-parser.yaml", FileFormat::Yaml),
      (".i18next-parser.toml", FileFormat::Toml),
      (".i18next-parser.ini", FileFormat::Ini),
      ("i18next-parser.json5", FileFormat::Json5),
      ("i18next-parser.json", FileFormat::Json),
      ("i18next-parser.yaml", FileFormat::Yaml),
      ("i18next-parser.toml", FileFormat::Toml),
      ("i18next-parser.ini", FileFormat::Ini),
    ];

    let mut found_config = false;
    for (file, format) in &config_files {
      log::trace!("Looking for {} in {}", file.italic().yellow(), working_dir.display().yellow());
      let file_name = &working_dir.join(file);
      let config_file: File<FileSourceFile, FileFormat> = file_name.clone().into();
      let source = config_file.format(*format).required(false);
      builder = builder.add_source(source);
      if file_name.exists() {
        found_config = true;
        log::info!("found {} in {}", file.italic().green(), working_dir.display().yellow());
      }
    }

    if !found_config {
      log::warn!("No configuration file found. Using default configuration.");
    }

    let configuration = builder.build().and_then(|config| config.try_deserialize())?;
    log::trace!("Loaded configuration: {:#?}", configuration);
    Ok(configuration)
  }

  /// Get the output destination for the i18n system.
  pub fn get_output(&self) -> String {
    self.working_dir.join(&self.output).to_str().unwrap().to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test_log::test]
  fn test_line_ending_auto() {
    let line_ending = LineEnding::Auto;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "auto".into());
  }

  #[test_log::test]
  fn test_line_ending_crlf() {
    let line_ending = LineEnding::Crlf;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "crlf".into());
  }

  #[test_log::test]
  fn test_line_ending_cr() {
    let line_ending = LineEnding::Cr;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "cr".into());
  }

  #[test_log::test]
  fn test_line_ending_lf() {
    let line_ending = LineEnding::Lf;
    let value: config::Value = line_ending.into();
    assert_eq!(value, "lf".into());
  }
}
#[cfg(test)]
mod config_tests {

  use super::*;

  #[test_log::test]
  fn config_default_values_are_correct() {
    let config = Config::default();
    assert_eq!(config.working_dir, PathBuf::from("."));
    assert_eq!(config.locales, vec!["en"]);
    assert!(config.input.contains(&"src/**/*.{ts,tsx}".into()));
    assert_eq!(config.output, ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR));
    assert!(!config.verbose);
  }

  #[test_log::test]
  fn config_new_sets_working_dir_and_verbose() {
    let working_dir = "/tmp";
    let verbose = true;
    let config = Config::new(working_dir, verbose).unwrap();
    assert_eq!(config.working_dir, PathBuf::from(working_dir));
    assert!(config.verbose);
  }

  #[test_log::test]
  fn config_get_output_constructs_correct_path() {
    let config = Config {
      working_dir: PathBuf::from("/tmp"),
      output: "locales/$LOCALE/$NAMESPACE.json".to_string(),
      ..Config::default()
    };
    let expected_output = "/tmp/locales/$LOCALE/$NAMESPACE.json";
    assert_eq!(config.get_output(), expected_output);
  }

  #[test_log::test]
  fn config_new_handles_invalid_working_dir() {
    let working_dir = "\0"; // Invalid path
    let verbose = false;
    assert!(Config::new(working_dir, verbose).is_ok());
  }
}
