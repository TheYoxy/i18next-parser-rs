use std::{collections::HashMap, path::PathBuf, str::FromStr};

use log::trace;
use serde_json::Value;

use crate::{
  catalog::get_catalog,
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

  let value = get_catalog(&path);

  let old_value = get_catalog(&backup);
  let old_value = old_value.as_ref();

  trace!("Value: {value:?} -> {old_value:?}");

  let ns_separator = &config.key_separator;
  let full_key_prefix = namespace.to_string() + ns_separator;
  let merged = merge_hashes(catalog, value.as_ref(), old_value, &full_key_prefix, false, config);
  let old_merged = merge_hashes(&merged.new, old_value, None, &full_key_prefix, false, &Config {
    keep_removed: false,
    ..Default::default()
  });
  let old_catalog = transfer_values(&merged.old, &old_merged.old);
  if config.verbose {
    print_counts(locale, namespace, unique_count, unique_plurals_count, &merged, &old_merged, config);
  }

  MergeResults { namespace: namespace.to_string(), locale: locale.to_string(), path, backup, merged, old_catalog }
}
