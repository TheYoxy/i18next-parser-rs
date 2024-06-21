use std::collections::HashMap;

use crate::config::Config;
use crate::helper::merge_hashes::MergeResult;

pub(crate) fn print_counts(
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
    println!("[{}] {}", locale, namespace);
    let unique_count = unique_count.get(namespace).unwrap_or(&0);
    let unique_plurals_count = unique_plurals_count.get(namespace).unwrap_or(&0);
    println!("Unique keys: {} ({} are plurals)", unique_count, unique_plurals_count);
    let add_count = unique_count.saturating_sub(merge_count);
    println!("Added keys: {}", add_count);
    println!("Restored keys: {}", restore_count);
    if config.keep_removed {
        println!("Unreferenced keys: {}", old_count);
    } else {
        println!("Removed keys: {}", old_count);
    }
    if config.reset_default_value_locale.is_some() {
        println!("Reset keys: {}", reset_count);
    }
    println!();
}
