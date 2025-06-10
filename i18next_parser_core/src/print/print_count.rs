use std::collections::HashMap;

use color_eyre::owo_colors::OwoColorize;

use crate::helper::merge_hashes::MergeResult;

pub fn print_counts(
  locale: &str,
  namespace: &str,
  unique_count: &HashMap<String, usize>,
  unique_plurals_count: &HashMap<String, usize>,
  merged: &MergeResult,
  old_merged: &MergeResult,
) {
  let unique_count = unique_count.get(namespace).unwrap_or(&0);
  let unique_plurals_count = unique_plurals_count.get(namespace).unwrap_or(&0);

  let add_count = merged.pull_count;
  let modified_count = old_merged.merge_count;
  let deleted_count = merged.old_count;
  if add_count == 0 && modified_count == 0 && deleted_count == 0 {
    return;
  }

  let keys = format!("Keys: {}", unique_count.underline());
  let plurals = if *unique_plurals_count == 0 { "".into() } else { format!("({unique_plurals_count} are plurals)") };

  let diff = format!(
    "{}{} {}{} {}{}",
    "+".green(),
    add_count.green(),
    "~".yellow(),
    modified_count.yellow(),
    "-".red(),
    deleted_count.red()
  );

  tracing::info!(target: "count", "[{}] {} {} {} {}", locale.cyan().italic(), namespace.blue(), keys, diff, plurals.bright_black());
}
