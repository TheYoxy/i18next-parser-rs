//! This module provides functionality for reading files into serde values
use std::{fs::File, io::BufReader, path::PathBuf};

use color_eyre::owo_colors::OwoColorize;
use log::{trace, warn};
use serde_json::Value;

/// Read a file into a serde value
pub fn read_file_into_serde(path: &PathBuf) -> Option<Value> {
  trace!("Reading file: {}", path.display().yellow());
  let file = File::open(path);
  if file.is_err() && path.file_name().and_then(|f| f.to_str()).is_some_and(|name| !name.to_string().contains("_old")) {
    warn!("Unable to find file: {}", path.display().yellow());
  }
  file.map_or(Default::default(), |file| {
    let reader = BufReader::new(file);
    if path.extension().is_some_and(|ext| ext == "yml") {
      serde_yaml_ng::from_reader(reader).ok()
    } else {
      // read json file
      serde_json::from_reader(reader).ok()
    }
  })
}

#[cfg(test)]
mod tests {
  use tempdir::TempDir;

  use super::*;

  const BASE_PATH: &str = "assets/locales/";

  #[test_log::test]
  fn test_get_catalog_with_existing_json_file() {
    let dir = TempDir::new("test").unwrap();
    {
      let path = dir.path().join("en").join("default.json");
      let content = r#"{
        "key1": "value1",
        "key2": "value2"
      }"#;
      std::fs::create_dir_all(path.parent().unwrap()).unwrap();
      std::fs::write(&path, content).unwrap();
    }
    let path = dir.path().join("en").join("default.json");
    let catalog = read_file_into_serde(&path);
    assert!(catalog.is_some());
    let catalog_value = catalog.unwrap();
    assert_eq!(catalog_value["key1"], "value1");
    assert_eq!(catalog_value["key2"], "value2");
    drop(dir);
  }

  #[test_log::test]
  fn test_get_catalog_with_existing_yaml_file() {
    let dir = TempDir::new("test").unwrap();
    {
      let path = dir.path().join("en").join("default.yml");
      let content = r#"key3: value3
key4: value4
"#;
      std::fs::create_dir_all(path.parent().unwrap()).unwrap();
      std::fs::write(&path, content).unwrap();
    }

    let path = dir.path().join("en").join("default.yml");
    let catalog = read_file_into_serde(&path);
    assert!(catalog.is_some());
    let catalog_value = catalog.unwrap();
    assert_eq!(catalog_value["key3"], "value3");
    assert_eq!(catalog_value["key4"], "value4");
  }

  #[test_log::test]
  fn test_get_catalog_with_non_existing_file() {
    let path = PathBuf::from(BASE_PATH.to_owned() + "en/non_existing.json");
    let catalog = read_file_into_serde(&path);
    assert!(catalog.is_none());
  }
}
