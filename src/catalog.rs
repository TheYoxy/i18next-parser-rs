use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use log::{debug, trace, warn};
use serde_json::Value;

pub fn get_catalog(path: &PathBuf) -> Option<Value> {
  trace!("Reading file: {:?}", path);
  let file = File::open(path);
  if file.is_err() && path.file_name().and_then(|f| f.to_str()).is_some_and(|name| !name.to_string().contains("_old")) {
    warn!("Unable to find file: {:?}", path);
  }
  file.map_or(Default::default(), |file| {
    let reader = BufReader::new(file);
    if path.extension().is_some_and(|ext| ext == "yml") {
      serde_yaml::from_reader(reader).ok()
    } else {
      // read json file
      serde_json::from_reader(reader).ok()
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  const BASE_PATH: &str = "assets/locales/";

  #[test]
  fn test_get_catalog_with_existing_json_file() {
    let path = PathBuf::from(BASE_PATH.to_owned() + "en/default.json");
    let catalog = get_catalog(&path);
    assert!(catalog.is_some());
    let catalog_value = catalog.unwrap();
    assert_eq!(catalog_value["key1"], "value1");
    assert_eq!(catalog_value["key2"], "value2");
  }

  #[test]
  fn test_get_catalog_with_existing_yaml_file() {
    let path = PathBuf::from(BASE_PATH.to_owned() + "en/default.yml");
    let catalog = get_catalog(&path);
    assert!(catalog.is_some());
    let catalog_value = catalog.unwrap();
    assert_eq!(catalog_value["key3"], "value3");
    assert_eq!(catalog_value["key4"], "value4");
  }

  #[test]
  fn test_get_catalog_with_non_existing_file() {
    let path = PathBuf::from(BASE_PATH.to_owned() + "en/non_existing.json");
    let catalog = get_catalog(&path);
    assert!(catalog.is_none());
  }
}
