use serde_json::Value;

use crate::config::Config;
use crate::file::{merge_results, MergeResults};
use crate::log_time;
use crate::transform::transform_entries::{transform_entries, TransformEntriesResult};
use crate::visitor::Entry;

pub(crate) fn merge_all_values(entries: Vec<Entry>, config: &Config) -> Vec<MergeResults> {
  log_time!("Preparing entries to write", || {
    let locales = &config.locales;
    locales
      .iter()
      .filter_map(|locale| {
        let TransformEntriesResult { unique_count, unique_plurals_count, value } =
          transform_entries(&entries, locale, config);

        if let Value::Object(catalog) = value {
          Some(
            catalog
              .iter()
              .map(|(namespace, catalog)| {
                merge_results(locale, namespace, catalog, &unique_count, &unique_plurals_count, config)
              })
              .collect::<Vec<_>>(),
          )
        } else {
          None
        }
      })
      .flatten()
      .collect::<Vec<_>>()
  })
}
