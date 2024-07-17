//! This module contains the `IsEmpty` trait which is used to check if a `serde_json::Value` is empty.
use serde_json::Value;

/// Trait to check if a `serde_json::Value` is empty.
pub trait IsEmpty {
  fn is_empty(&self) -> bool;
}

/// Implement the `IsEmpty` trait for `serde_json::Value`.
impl IsEmpty for Value {
  fn is_empty(&self) -> bool {
    match self {
      Value::Null => true,
      Value::Bool(_) => false,
      Value::Number(_) => false,
      Value::String(_) => false,
      Value::Array(arr) => arr.is_empty(),
      Value::Object(obj) => obj.is_empty(),
    }
  }
}
#[cfg(test)]
mod tests {
  use serde_json::{json, Value};

  use super::*;

  /// Checks if a `null` value is correctly identified as empty.
  #[test]
  fn null_value_is_empty() {
    let value = Value::Null;
    assert!(value.is_empty());
  }

  /// Checks if a non-empty string is correctly identified as not empty.
  #[test]
  fn non_empty_string_is_not_empty() {
    let value = Value::String("Hello".to_string());
    assert!(!value.is_empty());
  }

  /// Checks if an empty array is correctly identified as empty.
  #[test]
  fn empty_array_is_empty() {
    let value = Value::Array(vec![]);
    assert!(value.is_empty());
  }

  /// Checks if a non-empty array is correctly identified as not empty.
  #[test]
  fn non_empty_array_is_not_empty() {
    let value = Value::Array(vec![json!(1)]);
    assert!(!value.is_empty());
  }

  /// Checks if an empty object is correctly identified as empty.
  #[test]
  fn empty_object_is_empty() {
    let value = Value::Object(serde_json::Map::new());
    assert!(value.is_empty());
  }

  /// Checks if a non-empty object is correctly identified as not empty.
  #[test]
  fn non_empty_object_is_not_empty() {
    let mut map = serde_json::Map::new();
    map.insert("key".to_string(), json!("value"));
    let value = Value::Object(map);
    assert!(!value.is_empty());
  }

  /// Checks if a boolean value is correctly identified as not empty.
  #[test]
  fn boolean_value_is_not_empty() {
    let value = Value::Bool(true);
    assert!(!value.is_empty());
  }

  /// Checks if a number value is correctly identified as not empty.
  #[test]
  fn number_value_is_not_empty() {
    let value = Value::Number(serde_json::Number::from(42));
    assert!(!value.is_empty());
  }
}
