/// An enum for all plural rule categories.
#[derive(Clone, Copy, Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl From<&str> for PluralCategory {
    fn from(s: &str) -> Self {
        match s {
            "zero" => PluralCategory::Zero,
            "one" => PluralCategory::One,
            "two" => PluralCategory::Two,
            "few" => PluralCategory::Few,
            "many" => PluralCategory::Many,
            _ => PluralCategory::Other,
        }
    }
}
