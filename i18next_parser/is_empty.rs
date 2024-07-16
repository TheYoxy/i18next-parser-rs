//! This module contains the `IsEmpty` trait which is used to check if a `serde_json::Value` is empty.
use serde_json::Value;

/// Trait to check if a `serde_json::Value` is empty.
pub(crate) trait IsEmpty {
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
