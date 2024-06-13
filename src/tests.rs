#![cfg(test)]

use std::path::PathBuf;

use flatten_json_object::Flattener;
use serde_json::Value;
use tempdir::TempDir;

use crate::{
  file::{merge_all_results, MergeAllResults},
  is_empty::IsEmpty,
};
use crate::{
  transform::{transform_entries, TransformEntriesResult},
  utils::initialize_logging,
};
use crate::config::Config;
use crate::config::Options;
use crate::file::parse_directory;

#[test]
fn should_parse_successfully() -> color_eyre::Result<()> {
  initialize_logging()?;
  let working_path = "assets";
  let locales = vec!["en".to_string(), "fr".to_string()];
  let dir = TempDir::new("translations")?;

  let mut config = Config::new(working_path, false)?;
  config.locales = locales.into();
  config.output = dir.path().join("locales/$LOCALE/$NAMESPACE.json").to_str().map(|s| s.to_string()).unwrap();
  config.input = vec!["**/*.{ts,tsx}".to_string()];
  let config = &config;
  let options = Options::from(config);

  let entries = parse_directory(&PathBuf::from(working_path), config)?;
  for locale in options.locales.iter() {
    let TransformEntriesResult { unique_count, unique_plurals_count, value } =
      transform_entries(&entries, locale, &options);

    if let Value::Object(catalog) = value {
      println!("Catalog: {:?}", catalog);
      assert_eq!(catalog.len(), 1, "expected 1 namespace for locale {locale}");
      for (namespace, catalog) in catalog {
        let MergeAllResults { path: _path, backup: _backup, merged, old_catalog: _old_catalog } =
          merge_all_results(locale, &namespace, &catalog, &unique_count, &unique_plurals_count, &options);
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
    }
  }

  Ok(())
}
