use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) enum LineEnding {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
  pub(crate) working_dir: PathBuf,
  pub(crate) locales: Vec<String>,
  pub(crate) input: Vec<String>,
  pub(crate) output: String,
  pub(crate) context_separator: String,
  pub(crate) create_old_catalogs: bool,
  pub(crate) default_namespace: String,
  pub(crate) default_value: String,
  pub(crate) keep_removed: bool,
  pub(crate) key_separator: String,
  pub(crate) line_ending: LineEnding,
  pub(crate) namespace_separator: String,
  pub(crate) plural_separator: String,
  pub(crate) sort: bool,
  pub(crate) verbose: bool,
  pub(crate) fail_on_warnings: bool,
  pub(crate) fail_on_update: bool,
  pub(crate) reset_default_value_locale: Option<String>,
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
      output: "locales/$LOCALE/$NAMESPACE.json".into(),
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

impl Config {
  pub(crate) fn new<T>(working_dir: T, verbose: bool) -> Result<Self, config::ConfigError>
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

    builder.build().and_then(|config| config.try_deserialize())
  }

  pub(crate) fn get_output(&self) -> String {
    self.working_dir.join(&self.output).to_str().unwrap().to_string()
  }
}
