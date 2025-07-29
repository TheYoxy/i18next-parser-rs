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
      Some(Value::Array(v)) => {
        if v.len() == 1 {
          v.first().and_then(|v| v.as_str().map(|s| s.to_string()))
        } else {
          None
        }
      },
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
pub trait SerdeVecHelper {
  fn values_to_value(self) -> Option<Value>;
}

impl SerdeVecHelper for Vec<Value> {
  #[inline]
  fn values_to_value(self) -> Option<Value> {
    if self.is_empty() {
      None
    } else {
      let mut values = vec![];
      for v in self {
        values.push(v);
      }
      if values.is_empty() { None } else { Some(Value::Array(values)) }
    }
  }
}
