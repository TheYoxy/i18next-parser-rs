use crate::visitor::I18NextOptions;

#[derive(Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct Location {
  pub file: String,
  pub start: usize,
  pub end: usize,
}

impl std::fmt::Display for Location {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}:{}", self.file, self.start, self.end)
  }
}

impl Location {
  pub fn new(file: String, start: usize, end: usize) -> Self {
    Self { file, start, end }
  }

  #[cfg(not(feature = "print_error_location"))]
  pub fn print(&self) {
  }

  #[cfg(feature = "print_error_location")]
  #[tracing::instrument(skip(self))]
  pub fn print(&self) {
    use bat::{
      line_range::{LineRange, LineRanges},
      PrettyPrinter,
    };
    let content = std::fs::read_to_string(&self.file).unwrap();
    let mut start_line = 1;
    let mut end_line = 1;
    let start_pos = self.start;
    let end_pos = self.end;
    for (i, c) in content.chars().enumerate() {
      if i == start_pos {
        start_line = end_line;
      }
      if i == end_pos {
        break;
      }

      if c == '\n' {
        end_line += 1;
      }
    }

    let bound = 2;
    let range = LineRange::from(format!("{}:{}", start_line - bound, end_line + bound).as_str()).unwrap();
    PrettyPrinter::new()
      .input_file(&self.file)
      .line_ranges(LineRanges::from(vec![range]))
      .header(true)
      .grid(true)
      .line_numbers(true)
      .highlight_range(start_line, end_line)
      .print()
      .unwrap();
  }
}

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
  pub location: Location,
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
  /// the context found for the key
  pub context: Option<Vec<String>>,
}

/// Implement the `PartialEq` trait for `Entry`.
impl PartialEq for Entry {
  /// Compare two entries.
  fn eq(&self, other: &Self) -> bool {
    self.key == other.key
      && self.value == other.value
      && self.namespace == other.namespace
      && self.context == other.context
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

  pub fn new_with_context<Key: Into<String>, Value: Into<String>, Ns: Into<String>>(
    key: Key,
    value: Value,
    namespace: Ns,
    context: Vec<String>,
  ) -> Self {
    Self {
      key: key.into(),
      value: Some(value.into()),
      namespace: Some(namespace.into()),
      context: Some(context),
      ..Default::default()
    }
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
