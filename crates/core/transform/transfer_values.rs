//! Transfers values from a source JSON Value to a target JSON Value.
use serde_json::Value;

/// Transfers values from a source JSON Value to a target JSON Value.
///
/// If both the source and target are JSON objects, this function will iterate over the source object.
/// For each key-value pair in the source object, if the key does not exist in the target object, it will be added.
/// If the key does exist, the function will recursively call itself with the source and target values for that key.
///
/// If the source and target are not both objects, the target is returned as is.
///
/// # Arguments
///
/// * `source` - A reference to the source JSON Value.
/// * `target` - A reference to the target JSON Value.
///
/// # Returns
///
/// * `Value` - The target JSON Value after transferring values from the source.
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
