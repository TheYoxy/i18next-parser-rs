#![cfg(test)]

use std::path::PathBuf;
use std::{io::Write, path::Path};

use flatten_json_object::Flattener;
use pretty_assertions::assert_eq;
use serde::Serialize;
use serde_json::{json, Value};
use tempdir::TempDir;

use crate::config::Config;
use crate::file::{parse_directory, prepare_to_write, write_to_file};
use crate::utils::initialize_logging;
use crate::{file::MergeAllResults, is_empty::IsEmpty};

fn setup_test(path: Option<&str>) -> color_eyre::Result<(&str, Config)> {
  let _ = initialize_logging();

  let working_path = path.unwrap_or("assets");
  let locales = vec!["en".to_string(), "fr".to_string()];

  let mut config = Config::new(working_path, false)?;
  config.locales = locales;
  config.output =
    PathBuf::from(working_path).join("locales/$LOCALE/$NAMESPACE.json").to_str().map(|s| s.to_string()).unwrap();
  config.input = vec!["**/*.{ts,tsx}".to_string()];

  Ok((working_path, config))
}

#[test]
fn should_parse_successfully() -> color_eyre::Result<()> {
  let (working_path, config) = &setup_test(None)?;

  let entries = parse_directory(&PathBuf::from(working_path), config)?;

  let entries = prepare_to_write(entries, config)?;
  for entry in entries {
    let MergeAllResults { locale: _locale, path: _path, backup: _backup, merged, old_catalog: _old_catalog } = entry;

    assert_eq!(merged.old_count, 0, "there isn't any values yet");
    assert_eq!(merged.merge_count, 0, "there is 0 values to merge");
    assert_eq!(merged.pull_count, 0, "there is 8 new values");
    assert_eq!(merged.reset_count, 0, "there is 0 values to reset");
    assert!(merged.old.is_empty(), "there isn't any old values");
    assert!(merged.reset.is_empty(), "there isn't any reset values");
    assert!(!merged.new.is_empty(), "values must be parsed");
    let new = Flattener::new().set_key_separator(".").flatten(&merged.new)?;
    let new = new.as_object().unwrap();
    println!("New: {:?}", new);
    for key in [
      "toast.title",
      "toast.validation.error",
      "toast.text.success",
      "toast.text.error",
      "dialog.title",
      "dialog.description",
      "button.clear",
      "button.submit",
    ] {
      assert!(new.contains_key(key), "the key {key:?} is present in the new catalog");
    }
  }

  Ok(())
}

fn validate_object<T: Into<Value>>(v: &Value, key: &str, value: T) {
  if let Value::Object(v) = v {
    if let Some((key, remaining)) = key.split_once('.') {
      let v = v.get(key).unwrap();
      validate_object(v, remaining, value)
    }
  } else {
    assert_eq!(v, &value.into());
  }
}

fn create_file<P: AsRef<Path>, V: ?Sized + Serialize>(path: P, value: &V) -> color_eyre::Result<()> {
  std::fs::create_dir_all(path.as_ref().parent().unwrap())?;
  let file = std::fs::File::create(path)?;
  serde_json::to_writer_pretty(file, value)?;

  Ok(())
}

#[test]
fn should_not_override_current_values() -> color_eyre::Result<()> {
  let source_text = r#"const el = <Trans ns="ns" i18nKey="dialog.title">Reset password</Trans>;"#;
  let dir = TempDir::new("translations")?;
  let file = dir.path().join("text.tsx");
  let mut file = std::fs::File::create(file)?;
  file.write_all(source_text.as_bytes())?;

  let en = dir.path().join("locales/en/ns.json");
  create_file(en, &(json!({ "dialog": { "title": "Reset" }})))?;

  let fr = dir.path().join("locales/fr/ns.json");
  create_file(fr, &(json!({ "dialog": { "title": "[fr] Reset" }})))?;

  let path = dir.path().to_str().unwrap();
  let (working_path, config) = &setup_test(Some(path))?;
  let config = &Config { locales: vec!["en".to_string(), "fr".to_string()], ..config.clone() };
  let entries = parse_directory(&PathBuf::from(working_path), config)?;

  write_to_file(entries, config)?;

  let en = dir.path().join("locales/en/ns.json");
  let en = std::fs::read_to_string(en)?;
  let en = serde_json::from_str::<Value>(&en)?;

  let fr = dir.path().join("locales/fr/ns.json");
  let fr = std::fs::read_to_string(fr)?;
  let fr = serde_json::from_str::<Value>(&fr)?;

  assert!(en.is_object());
  validate_object(&en, "dialog.title", "Reset");

  assert!(fr.is_object());
  validate_object(&fr, "dialog.title", "[fr] Reset");

  Ok(())
}
