use crate::models::conflict_entry::ConflictEntry;

/// Enum representing the type of conflict that can occur when converting a dot path to a hash.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Conflict {
  Value(ConflictEntry, ConflictEntry),
}
