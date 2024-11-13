use std::collections::HashMap;

use tracing::info;

use crate::{config::Config, helper::merge_hashes::MergeResult};

pub fn print_counts(
  locale: &str,
  namespace: &str,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  merged: &MergeResult,
  old_merged: &MergeResult,
  config: &Config,
) {
  let merge_count = merged.merge_count;
  let restore_count = old_merged.merge_count;
  let old_count = merged.old_count;
  let reset_count = merged.reset_count;
  info!(layer = "count", "[{}] {}", locale, namespace);
  let unique_count = unique_count.get(namespace).unwrap_or(&0);
  let unique_plurals_count = unique_plurals_count.get(namespace).unwrap_or(&0);
  info!(layer = "count", "Unique keys: {} ({} are plurals)", unique_count, unique_plurals_count);
  let add_count = unique_count.saturating_sub(merge_count);
  info!(layer = "count", "Added keys: {}", add_count);
  info!(layer = "count", "Restored keys: {}", restore_count);
  if config.keep_removed {
    info!(layer = "count", "Unreferenced keys: {}", old_count);
  } else {
    info!(layer = "count", "Removed keys: {}", old_count);
  }
  if config.reset_default_value_locale.is_some() {
    info!(layer = "count", "Reset keys: {}", reset_count);
  }
  info!(layer = "count", "");
}
