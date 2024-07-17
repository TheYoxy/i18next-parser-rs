use crate::visitor::I18NextOptions;

/// This struct represents an entry in the i18n system.
///
/// # Fields
///
/// * `key` - The key of the entry.
/// * `value` - The value found for the key.
/// * `namespace` - The namespace found for the key.
/// * `i18next_options` - All i18next options found in the file.
/// * `has_count` - A boolean indicating whether the key has a count (if plural).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Entry {
  /// the key of the entry
  pub key: String,
  /// the value found for the key
  pub value: Option<String>,
  /// the namespace found for the key
  pub namespace: Option<String>,
  /// all i18next options found in the file
  pub i18next_options: Option<I18NextOptions>,
  /// the count found for the key (if plural)
  pub has_count: bool,
}
