#![cfg(test)]

use std::{
  collections::HashMap,
  io::Write,
  path::{Path, PathBuf, MAIN_SEPARATOR_STR},
};

use clap::Parser;
use color_eyre::{eyre::eyre, owo_colors::OwoColorize};
use flatten_json_object::Flattener;
use log::debug;
use pretty_assertions::{assert_eq, assert_str_eq};
use serde::Serialize;
use serde_json::{json, Value};
use tempdir::TempDir;

use crate::{
  cli::{Cli, Runnable},
  config::Config,
  is_empty::IsEmpty,
  merger::{merge_all_values::merge_all_values, merge_results::MergeResults},
  parser::parse_directory::parse_directory,
  utils::initialize_logging,
};

fn setup_test(path: Option<&str>) -> color_eyre::Result<(&str, Config)> {
  let _ = initialize_logging();

  let working_path = path.unwrap_or("assets");

  let mut config = Config::new(working_path, false)?;
  config.locales = vec!["en".into(), "fr".into()];
  config.output = [working_path, "locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR);
  config.input = vec!["**/*.{ts,tsx}".into()];

  Ok((working_path, config))
}

#[test_log::test]
fn should_parse_successfully() -> color_eyre::Result<()> {
  let (working_path, config) = &setup_test(None)?;

  let entries = parse_directory(PathBuf::from(working_path), config)?;

  let entries = merge_all_values(entries, config)?;
  for entry in entries {
    let MergeResults {
      namespace: _namespace,
      locale: _locale,
      path: _path,
      backup: _backup,
      merged,
      old_catalog: _old_catalog,
    } = entry;

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

fn validate_object<T: Into<Value>>(key: &str, current_value: &Value, to_compare: T) {
  if let Value::Object(v) = current_value {
    if let Some((key, remaining)) = key.split_once('.') {
      let v = v.get(key).unwrap();
      validate_object(remaining, v, to_compare)
    } else {
      let v = v.get(key).unwrap();
      validate_object(key, v, to_compare)
    }
  } else {
    let val = &to_compare.into();
    match (val, current_value) {
      (Value::String(val), Value::String(current_value)) => {
        assert_str_eq!(val, current_value, "there is a difference in the key {key:#?}")
      },
      (_, _) => assert_eq!(val, current_value, "there is a difference in the key {key:#?}"),
    }
  }
}

fn create_file<P: AsRef<Path>, V: ?Sized + Serialize>(path: P, value: &V) -> color_eyre::Result<()> {
  let path = path.as_ref();
  let parent = path.parent().ok_or(eyre!("unable to get parent of {}", path.display().yellow()))?;
  std::fs::create_dir_all(parent)?;
  let file = std::fs::File::create(path)?;
  serde_json::to_writer_pretty(file, value)?;
  debug!("{} written", path.display().yellow());

  Ok(())
}

#[test]
fn should_not_override_current_values() -> color_eyre::Result<()> {
  let _ = initialize_logging();
  let dir = TempDir::new("translations")?;
  let mut map = HashMap::new();

  let raw_en = json!({ "dialog": { "title": "Reset" }});
  map.insert("en", raw_en);
  map.insert("fr", json!({ "dialog": { "title": "[fr] Reset" }}));
  fn create_files(dir: &TempDir, map: &HashMap<&str, Value>) -> color_eyre::Result<()> {
    let source_text = r#"const el = <Trans ns="ns" i18nKey="dialog.title">Reset password</Trans>;"#;
    let dir_path = dir.path();
    let file_path = dir_path.join("src/main.tsx");
    std::fs::create_dir_all(file_path.parent().unwrap())?;
    let mut file = std::fs::File::create(&file_path)?;
    file.write_all(source_text.as_bytes())?;
    debug!("{} written", file_path.display().yellow());

    let locales: Vec<String> = vec!["en".into(), "fr".into()];
    for lang in &locales {
      let file = dir_path.join("locales").join(lang).join("ns.json");
      let raw_val = map.get(lang.as_str()).ok_or(eyre!("Unable to get {} value", lang.yellow()))?;
      create_file(file, raw_val)?;
    }

    let fr = dir_path.join("locales/fr/ns.json");
    let raw_fr = json!({ "dialog": { "title": "[fr] Reset" }});
    create_file(fr, &raw_fr)?;

    let config = Config {
      locales,
      output: ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR),
      input: vec!["**/*.{ts,tsx}".into()],
      ..Default::default()
    };

    let config_file = dir_path.join(".i18next-parser.json");
    create_file(config_file, &config)?;
    Ok(())
  }

  create_files(&dir, &map)?;
  let args = ["", "-v", dir.path().to_str().unwrap()];
  let cli = Cli::parse_from(args);
  cli.run()?;

  let en: Value = {
    let en = dir.path().join("locales/en/ns.json");
    let en = std::fs::read(en)?;
    serde_json::from_slice(&en)?
  };
  validate_object("dialog.title", &en, "Reset password");
  assert_eq!(en, json!({ "dialog": { "title": "Reset password" } }));

  let fr: Value = {
    let fr = dir.path().join("locales/fr/ns.json");
    let fr = std::fs::read(fr)?;
    serde_json::from_slice(&fr)?
  };
  validate_object("dialog.title", &fr, "[fr] Reset");
  let raw_fr = map.get("fr").ok_or(eyre!("Unable to get fr value"))?;
  assert_eq!(fr, *raw_fr);

  drop(dir);
  Ok(())
}
