use serde_json::Value;

pub(crate) trait IsEmpty {
  fn is_empty(&self) -> bool;
}

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
