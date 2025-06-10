use serde_json::Value;

pub trait SerdeHelper {
  fn value_to_string(self) -> Option<String>;
  fn value_to_string_vec(self) -> Option<Vec<String>>;
}

impl SerdeHelper for Option<Value> {
  #[inline]
  fn value_to_string(self) -> Option<String> {
    match self {
      Some(Value::String(s)) => Some(s),
      _ => None,
    }
  }

  #[inline]
  fn value_to_string_vec(self) -> Option<Vec<String>> {
    match self {
      Some(Value::String(v)) => Some(vec![v]),
      Some(Value::Array(arr)) => Some(arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
      _ => None,
    }
  }
}
