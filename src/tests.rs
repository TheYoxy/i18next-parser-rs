#![cfg(test)]

use flatten_json_object::Flattener;
use serde_json::Value;

use crate::is_empty::IsEmpty;
use crate::{
  config::Options, merge_all_results, parse_file, transform_entries, MergeAllResults, TransformEntriesResult,
};

#[test]
fn should_parse() {
  let name = "assets/file.tsx";
  let locales = ["en", "fr"];
  let output = "tmp/locales/$LOCALE/$NAMESPACE.json";
  let entries = parse_file(name).unwrap();

  let options = Options::default();

  assert_eq!(locales.len(), 2);
  for locale in locales.iter() {
    let TransformEntriesResult { unique_count, unique_plurals_count, value } = transform_entries(&entries, &options);

    if let Value::Object(catalog) = value {
      println!("{:?}", catalog);
      assert_eq!(catalog.len(), 1, "expected 1 namespace for locale {locale}");
      for (namespace, catalog) in catalog {
        let MergeAllResults { path: _path, backup: _backup, merged, old_catalog: _old_catalog } =
          merge_all_results(locale, &namespace, &catalog, output, &unique_count, &unique_plurals_count, &options);
        assert_eq!(merged.old_count, 0, "there isn't any values yet");
        assert_eq!(merged.merge_count, 0, "there is 0 values to merge");
        assert_eq!(merged.pull_count, 0, "there is 8 new values");
        assert_eq!(merged.reset_count, 0, "there is 0 values to reset");
        assert!(merged.old.is_empty(), "there isn't any old values");
        assert!(merged.reset.is_empty(), "there isn't any reset values");
        assert!(!merged.new.is_empty(), "values must be parsed");
        let new = Flattener::new().set_key_separator(".").flatten(&merged.new).unwrap();
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
}
