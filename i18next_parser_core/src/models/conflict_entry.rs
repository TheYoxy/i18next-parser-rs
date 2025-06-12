use crate::Location;

/// Reprensents a conflict entry.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConflictEntry {
  /// The value that conflicts.
  pub value: String,
  /// The location of the conflict.
  pub location: Location,
}
impl From<&str> for ConflictEntry {
  fn from(value: &str) -> Self {
    Self { value: value.to_string(), location: Location::default() }
  }
}
impl From<String> for ConflictEntry {
  fn from(value: String) -> Self {
    Self { value, location: Location::default() }
  }
}

impl ConflictEntry {
  /// Default constructor for ConflictEntry.
  pub fn new(key: String, location: Location) -> Self {
    Self { value: key, location }
  }
}
