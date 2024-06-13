#![cfg(test)]

use flatten_json_object::Flattener;
use serde_json::Value;

use crate::utils::initialize_logging;
use crate::{config::Config, file::parse_file};
use crate::{config::Options, transform_entries, TransformEntriesResult};
use crate::{
  file::{merge_all_results, MergeAllResults},
  is_empty::IsEmpty,
};

#[test]
fn should_parse() -> color_eyre::Result<()> {
  initialize_logging()?;
  let name = "assets/file.tsx";
  let locales = ["en", "fr"];
  let entries = parse_file(name)?;
  let config = Config::new(name, false)?;
  let options = Options::from(config);

  assert_eq!(locales.len(), 2);
  for locale in locales.iter() {
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
