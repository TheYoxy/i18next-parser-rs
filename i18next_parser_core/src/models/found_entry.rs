use crate::Location;

#[derive(Debug, Clone)]
pub struct FoundEntry {
  pub value: String,
  pub location: Location,
}

impl FoundEntry {
  #[allow(dead_code)]
  pub fn new<T>(value: T) -> Self
  where
    T: Into<String>,
  {
    Self {
      value: value.into(),
      location: Location::default(),
    }
  }

  pub fn new_with_location<T>(value: T, location: Location) -> Self
  where
    T: Into<String>,
  {
    Self {
      value: value.into(),
      location,
    }
  }
}

impl PartialEq for FoundEntry {
  fn eq(&self, other: &Self) -> bool {
    self.value == other.value && self.location == other.location
  }
}
impl Eq for FoundEntry {}
impl From<&str> for FoundEntry {
  fn from(value: &str) -> Self {
    Self {
      value: value.to_string(),
      location: Location::default(),
    }
  }
}
impl From<String> for FoundEntry {
  fn from(value: String) -> Self {
    Self {
      value,
      location: Location::default(),
    }
  }
}
