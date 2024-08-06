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
#[derive(Debug, Default, Eq)]
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

/// Implement the `PartialEq` trait for `Entry`.
impl PartialEq for Entry {
  /// Compare two entries.
  fn eq(&self, other: &Self) -> bool {
    self.key == other.key && self.value == other.value && self.namespace == other.namespace
  }
}
impl Entry {
  /// Create a new entry.
  pub fn empty<Key: Into<String>>(key: Key) -> Self {
    Self { key: key.into(), ..Default::default() }
  }

  /// Create a new entry with a value and a namespace.
  pub fn new<Key: Into<String>, Value: Into<String>, Ns: Into<String>>(key: Key, value: Value, namespace: Ns) -> Self {
    Self { key: key.into(), value: Some(value.into()), namespace: Some(namespace.into()), ..Default::default() }
  }

  /// Create a new entry with a value.
  pub fn new_with_value<Key: Into<String>, Value: Into<String>>(key: Key, value: Value) -> Self {
    Self { key: key.into(), value: Some(value.into()), ..Default::default() }
  }

  /// Create a new entry with a namespace.
  pub fn new_with_ns<Key: Into<String>, Ns: Into<String>>(key: Key, namespace: Ns) -> Self {
    Self { key: key.into(), namespace: Some(namespace.into()), ..Default::default() }
  }
}
