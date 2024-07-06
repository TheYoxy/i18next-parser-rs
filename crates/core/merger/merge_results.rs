use std::{collections::HashMap, path::PathBuf, str::FromStr};

use log::trace;
use serde_json::Value;

use crate::{
  catalog::read_file_into_serde,
  config::Config,
  helper::merge_hashes::{merge_hashes, MergeResult},
  print::print_count::print_counts,
  transform::transfer_values::transfer_values,
};

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct MergeResults {
  pub(crate) namespace: String,
  pub(crate) locale: String,
  pub(crate) path: PathBuf,
  pub(crate) backup: PathBuf,
  pub(crate) merged: MergeResult,
  pub(crate) old_catalog: Value,
}

pub(crate) fn merge_results<C: AsRef<Config>>(
  locale: &str,
  namespace: &str,
  catalog: &Value,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  is_default: bool,
  config: C,
) -> MergeResults {
  let config = config.as_ref();
  let output = config.get_output();
  let path = output.replace("$LOCALE", locale).replace("$NAMESPACE", namespace);
  trace!("Path for output {output:?}: {path:?}");
  let path = PathBuf::from_str(&path).unwrap_or_else(|_| panic!("Unable to find path {path:?}"));
  // get backup file name
  let filename = {
    let filename = path.file_stem().and_then(|o| o.to_str()).unwrap_or_default();
    let extension = path.extension().and_then(|o| o.to_str()).unwrap_or_default();
    format!("{}_old.{}", filename, extension)
  };
  let backup = path.with_file_name(filename);
  trace!("File path: {path:?}");
  trace!("Backup path: {backup:?}");

  let value = read_file_into_serde(&path);

  let old_value = read_file_into_serde(&backup);
  let old_value = old_value.as_ref();

  trace!("Value: {value:?} -> {old_value:?}");

  let ns_separator = &config.key_separator;
  let full_key_prefix = namespace.to_string() + ns_separator;
  let merged = merge_hashes(catalog, value.as_ref(), old_value, &full_key_prefix, is_default, config);
  let old_merged = merge_hashes(
    &merged.new,
    old_value,
    None,
    &full_key_prefix,
    false,
    &Config { keep_removed: false, ..Default::default() },
  );
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if config.verbose {
    print_counts(locale, namespace, unique_count, unique_plurals_count, &merged, &old_merged, config);
  }

  MergeResults { namespace: namespace.to_string(), locale: locale.to_string(), path, backup, merged, old_catalog }
}

#[cfg(test)]
mod tests {
  use color_eyre::eyre::eyre;
  use log::info;
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use tempdir::TempDir;

  use super::*;

  fn init_test(ns: &str, locale: &str, value: &Value) -> color_eyre::Result<String> {
    let dir = TempDir::new("merge_results").unwrap();
    std::fs::create_dir_all(dir.path())?;
    let output = &dir.path().join("locales");
    let output = output.join(ns).join(format!("{locale}.json"));
    std::fs::create_dir_all(output.parent().unwrap())?;
    info!("Opening file {output:?}");
    let file = std::fs::File::create(&output)?;

    serde_json::to_writer_pretty(file, &value)?;
    info!("{value:?} -> {output:?}");

    output.to_str().ok_or(eyre!("Unable to get path")).map(|s| s.to_string())
  }

  #[test]
  fn merge_results_should_not_override_defaults() {
    let value = json!( {
      "key": "default_value"
    });

    let locale = "en";
    let namespace = "default";
    let output = init_test(locale, namespace, &value).unwrap();
    let catalog = json!({
        "key": "value"
    });
    let unique_count = HashMap::<String, usize>::new();
    let unique_plurals_count = HashMap::<String, usize>::new();
    let is_default = true;
    let config = Config { locales: vec![locale.into()], output, ..Default::default() };

    let result = merge_results(locale, namespace, &catalog, &unique_count, &unique_plurals_count, is_default, config);
    println!("Results: {:#?}", result);
    assert_eq!(result.merged.new, catalog);
    assert_eq!(result.merged.old, value);

    assert_eq!(result.merged.merge_count, 0);
  }
}
