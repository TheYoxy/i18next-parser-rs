use serde_json::Value;

pub(crate) fn transfer_values(source: &Value, target: &Value) -> Value {
  if let (Value::Object(source_map), Value::Object(target_map)) = (source, target) {
    let mut new_target_map = target_map.clone();
    for (key, source_value) in source_map {
      if !new_target_map.contains_key(key) {
        new_target_map.insert(key.clone(), source_value.clone());
      } else {
        let target_value = new_target_map.get_mut(key).unwrap();
        let transferred_value = transfer_values(source_value, target_value);
        *target_value = transferred_value;
      }
    }
    Value::Object(new_target_map)
  } else {
    target.clone()
  }
}
