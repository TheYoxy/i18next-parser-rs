//! This module contains the plural rules and resolver.
use std::collections::HashMap;

use color_eyre::{eyre::eyre, Result};
use intl_pluralrules::{PluralRuleType, PluralRules};

/// Cleans the provided code by replacing underscores with hyphens.
///
/// # Arguments
///
/// * `code` - A string slice that holds the code to be cleaned.
///
/// # Returns
///
/// * A String containing the cleaned code.
fn get_cleaned_code(code: &str) -> String {
  if code.contains('_') {
    return code.replace('_', "-");
  }
  code.to_string()
}

/// A struct representing a set of plural rules.
///
/// # Fields
///
/// * `lngs` - A vector of static string slices representing languages.
/// * `nr` - A vector of unsigned 32-bit integers representing numbers.
/// * `fc` - An unsigned 32-bit integer representing a function code.
struct PluralSet {
  /// A vector of static string slices representing languages.
  lngs: Vec<&'static str>,
  /// A vector of unsigned 32-bit integers representing numbers.
  nr: Vec<u32>,
  /// An unsigned 32-bit integer representing a function code.
  fc: u32,
}

/// Creates plural rules from a vector of PluralSet and adds them to the provided rules.
///
/// # Arguments
///
/// * `sets` - A vector of PluralSet.
fn create_rules(sets: Vec<PluralSet>) -> HashMap<&'static str, RuleValue> {
  let mut rules = Rules::new();
  let plural_funcs: Vec<fn(u32) -> u32> = vec![
    |n| (n > 1) as u32,  // 1
    |n| (n != 1) as u32, // 2
    |_| 0,               // 3
    |n| {
      (n % 10 == 1 && n % 100 != 11) as u32 + (n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20)) as u32 * 2
    }, // 4
    |n| {
      if n == 0 {
        0
      } else if n == 1 {
        1
      } else if n == 2 {
        2
      } else if n % 100 >= 3 && n % 100 <= 10 {
        3
      } else if n % 100 >= 11 {
        4
      } else {
        5
      }
    }, // 5
    |n| {
      if n == 1 {
        0
      } else if (2..=4).contains(&n) {
        1
      } else {
        2
      }
    }, // 6
    |n| {
      if n == 1 {
        0
      } else if n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20) {
        1
      } else {
        2
      }
    }, // 7
    |n| {
      if n == 1 {
        0
      } else if n == 2 {
        1
      } else if n != 8 && n != 11 {
        2
      } else {
        3
      }
    }, // 8
    |n| (n >= 2) as u32, // 9
    |n| {
      if n == 1 {
        0
      } else if n == 2 {
        1
      } else if n < 7 {
        2
      } else if n < 11 {
        3
      } else {
        4
      }
    }, // 10
    |n| {
      if n == 1 || n == 11 {
        0
      } else if n == 2 || n == 12 {
        1
      } else if n > 2 && n < 20 {
        2
      } else {
        3
      }
    }, // 11
    |n| (n % 10 != 1 || n % 100 == 11) as u32, // 12
    |n| (n != 0) as u32, // 13
    |n| {
      if n == 1 {
        0
      } else if n == 2 {
        1
      } else if n == 3 {
        2
      } else {
        3
      }
    }, // 14
    |n| {
      if n % 10 == 1 && n % 100 != 11 {
        0
      } else if n % 10 >= 2 && (n % 100 < 10 || n % 100 >= 20) {
        1
      } else {
        2
      }
    }, // 15
    |n| {
      if n % 10 == 1 && n % 100 != 11 {
        0
      } else if n != 0 {
        1
      } else {
        2
      }
    }, // 16
    |n| if n == 1 || (n % 10 == 1 && n % 100 != 11) { 0 } else { 1 }, // 17
    |n| {
      if n == 0 {
        0
      } else if n == 1 {
        1
      } else {
        2
      }
    }, // 18
    |n| {
      if n == 1 {
        0
      } else if n == 0 || (n % 100 > 1 && n % 100 < 11) {
        1
      } else if n % 100 > 10 && n % 100 < 20 {
        2
      } else {
        3
      }
    }, // 19
    |n| {
      if n == 1 {
        0
      } else if n == 0 || (n % 100 > 0 && n % 100 < 20) {
        1
      } else {
        2
      }
    }, // 20
    |n| {
      if n % 100 == 1 {
        1
      } else if n % 100 == 2 {
        2
      } else if n % 100 == 3 || n % 100 == 4 {
        3
      } else {
        0
      }
    }, // 21
    |n| {
      if n == 1 {
        0
      } else if n == 2 {
        1
      } else if !(0..=10).contains(&n) && n % 10 == 0 {
        2
      } else {
        3
      }
    }, // 22
  ];

  for set in sets {
    let func = plural_funcs[(set.fc - 1) as usize];
    for &lng in &set.lngs {
      rules.insert(lng, (set.nr.clone(), func));
    }
  }

  rules
}

/// A type alias for a tuple containing a vector of unsigned 32-bit integers and a function that takes an unsigned 32-bit integer and returns an unsigned 32-bit integer.
type RuleValue = (Vec<u32>, fn(u32) -> u32);

/// A type alias for a hashmap with static string slices as keys and RuleValue as values.
type Rules = HashMap<&'static str, RuleValue>;

/// A struct representing a plural resolver.
///
/// # Fields
///
/// * `rules` - A Rules hashmap containing the plural rules.
/// * `simplify_plural_suffix` - A boolean indicating whether to simplify the plural suffix.
pub(crate) struct PluralResolver {
  rules: Rules,
  simplify_plural_suffix: bool,
  prepend: Option<String>,
  version: I18NVersion,
}

/// A struct representing the supported i18n version.
#[derive(Default)]
pub(crate) enum I18NVersion {
  #[default]
  V4,
}

impl Default for PluralResolver {
  fn default() -> Self {
    Self::new(false, Some("_".to_string()), Default::default())
  }
}

impl PluralResolver {
  /// Returns a new PluralResolver with the provided simplify_plural_suffix value.
  ///
  /// # Arguments
  ///
  /// * `simplify_plural_suffix` - A boolean indicating whether to simplify the plural suffix.
  /// * `prepend` - An optional string slice that holds the value to prepend.
  pub(crate) fn new(simplify_plural_suffix: bool, prepend: Option<String>, version: I18NVersion) -> Self {
    let sets = vec![
      PluralSet {
        lngs: vec![
          "ach", "ak", "am", "arn", "br", "fil", "gun", "ln", "mfe", "mg", "mi", "oc", "pt", "pt-BR", "tg", "tl", "ti",
          "tr", "uz", "wa",
        ],
        nr: vec![1, 2],
        fc: 1,
      },
      PluralSet {
        lngs: vec![
          "af", "an", "ast", "az", "bg", "bn", "ca", "da", "de", "dev", "el", "en", "eo", "es", "et", "eu", "fi", "fo",
          "fur", "fy", "gl", "gu", "ha", "hi", "hu", "hy", "ia", "it", "kk", "kn", "ku", "lb", "mai", "ml", "mn", "mr",
          "nah", "nap", "nb", "ne", "nl", "nn", "no", "nso", "pa", "pap", "pms", "ps", "pt-PT", "rm", "sco", "se",
          "si", "so", "son", "sq", "sv", "sw", "ta", "te", "tk", "ur", "yo",
        ],
        nr: vec![1, 2],
        fc: 2,
      },
      PluralSet {
        lngs: vec![
          "ay", "bo", "cgg", "fa", "ht", "id", "ja", "jbo", "ka", "km", "ko", "ky", "lo", "ms", "sah", "su", "th",
          "tt", "ug", "vi", "wo", "zh",
        ],
        nr: vec![1],
        fc: 3,
      },
      PluralSet { lngs: vec!["be", "bs", "cnr", "dz", "hr", "ru", "sr", "uk"], nr: vec![1, 2, 5], fc: 4 },
      PluralSet { lngs: vec!["ar"], nr: vec![0, 1, 2, 3, 11, 100], fc: 5 },
      PluralSet { lngs: vec!["cs", "sk"], nr: vec![1, 2, 5], fc: 6 },
      PluralSet { lngs: vec!["csb", "pl"], nr: vec![1, 2, 5], fc: 7 },
      PluralSet { lngs: vec!["cy"], nr: vec![1, 2, 3, 8], fc: 8 },
      PluralSet { lngs: vec!["fr"], nr: vec![1, 2], fc: 9 },
      PluralSet { lngs: vec!["ga"], nr: vec![1, 2, 3, 7, 11], fc: 10 },
      PluralSet { lngs: vec!["gd"], nr: vec![1, 2, 3, 20], fc: 11 },
      PluralSet { lngs: vec!["is"], nr: vec![1, 2], fc: 12 },
      PluralSet { lngs: vec!["jv"], nr: vec![0, 1], fc: 13 },
      PluralSet { lngs: vec!["kw"], nr: vec![1, 2, 3, 4], fc: 14 },
      PluralSet { lngs: vec!["lt"], nr: vec![1, 2, 10], fc: 15 },
      PluralSet { lngs: vec!["lv"], nr: vec![1, 2, 0], fc: 16 },
      PluralSet { lngs: vec!["mk"], nr: vec![1, 2], fc: 17 },
      PluralSet { lngs: vec!["mnk"], nr: vec![0, 1, 2], fc: 18 },
      PluralSet { lngs: vec!["mt"], nr: vec![1, 2, 11, 20], fc: 19 },
      PluralSet { lngs: vec!["or"], nr: vec![2, 1], fc: 2 },
      PluralSet { lngs: vec!["ro"], nr: vec![1, 2, 20], fc: 20 },
      PluralSet { lngs: vec!["sl"], nr: vec![5, 1, 2, 3], fc: 21 },
      PluralSet { lngs: vec!["he", "iw"], nr: vec![1, 2, 20, 21], fc: 22 },
    ];

    let rules = create_rules(sets);

    Self { rules, simplify_plural_suffix, prepend, version }
  }

  /// Returns the plural rule for the provided code.
  ///
  /// # Arguments
  ///
  /// * `code` - A string slice that holds the code.
  ///
  /// # Returns
  ///
  /// * An Option containing a reference to a RuleValue.
  fn get_rule(&self, code: &str) -> Option<&RuleValue> {
    let cleaned_code = get_cleaned_code(code);
    self.rules.get(cleaned_code.as_str())
  }

  /// Returns a vector of strings representing the suffixes for the provided code.
  ///
  /// # Arguments
  ///
  /// * `code` - A string slice that holds the code.
  ///
  /// # Returns
  ///
  /// * A vector of Strings representing the suffixes.
  pub(crate) fn get_suffixes(&self, code: &str) -> Result<Vec<String>> {
    #[allow(unreachable_patterns)]
    match self.version {
      I18NVersion::V4 => {
        let lang: unic_langid::LanguageIdentifier = code.parse()?;
        let plural_rules = PluralRules::create(lang, PluralRuleType::CARDINAL).map_err(|e| eyre!(e))?;
        let result = plural_rules.resolved_options();
        let prepend = self.prepend.clone().unwrap_or_default();
        Ok(result.iter().map(|n| format!("{prepend}{n}")).collect::<Vec<String>>())
      },
      _ => {
        let result = match self.get_rule(code) {
          Some((numbers, _)) => numbers.iter().map(|&n| self.get_suffix(code, n)).collect(),
          None => vec![],
        };

        Ok(result)
      },
    }
  }

  /// Returns a string representing the suffix for the provided code and count.
  ///
  /// # Arguments
  ///
  /// * `code` - A string slice that holds the code.
  /// * `count` - An unsigned 32-bit integer representing the count.
  ///
  /// # Returns
  ///
  /// * A String representing the suffix.
  fn get_suffix(&self, code: &str, count: u32) -> String {
    match self.get_rule(code) {
      Some((rules, plural_func)) => {
        let idx = plural_func(count);
        if self.simplify_plural_suffix {
          match idx {
            1 => "".to_string(),
            2 => "plural".to_string(),
            _ => idx.to_string(),
          }
        } else {
          let rule = rules.get(idx as usize);
          fn return_suffix(prepend: Option<String>, suffix: Option<&u32>) -> String {
            match (prepend, suffix) {
              (Some(prepend), Some(suffix)) => format!("{prepend}{suffix}"),
              (None, Some(suffix)) => suffix.to_string(),
              _ => String::new(),
            }
          }

          return_suffix(self.prepend.clone(), rule)
        }
      },
      None => String::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod cleaned_code {
    use super::*;

    #[test_log::test]
    fn replaces_underscores_with_hyphens() {
      assert_eq!(get_cleaned_code("hello_world"), "hello-world");
      assert_eq!(get_cleaned_code("no_underscore"), "no-underscore");
    }

    #[test_log::test]
    fn returns_same_string_when_no_underscores() {
      assert_eq!(get_cleaned_code("helloworld"), "helloworld");
      assert_eq!(get_cleaned_code("nounderscore"), "nounderscore");
    }
  }

  mod plural_resolver {
    use super::*;

    #[test_log::test]
    fn plural_resolver_default_creates_new_with_simplified_suffix() {
      let resolver = PluralResolver::default();
      assert!(!resolver.simplify_plural_suffix);
    }

    #[test_log::test]
    fn plural_resolver_new_creates_new_with_given_simplify_suffix() {
      let resolver = PluralResolver::new(false, None, Default::default());
      assert!(!resolver.simplify_plural_suffix);
    }

    #[test_log::test]
    fn get_rule_returns_none_for_non_existent_code() {
      let resolver = PluralResolver::default();
      assert!(resolver.get_rule("nonexistent").is_none());
    }

    #[test_log::test]
    fn get_suffixes_return_elements_for_en() {
      let resolver = PluralResolver::default();
      let suffixes = resolver.get_suffixes("en");

      assert!(suffixes.is_ok());
      let suffixes = suffixes.unwrap();

      println!("{suffixes:?}");
      assert_eq!(suffixes.len(), 2);
      assert_eq!(suffixes, vec!["_one", "_other"]);
    }

    #[test_log::test]
    fn get_suffixes_return_elements_for_fr() {
      let resolver = PluralResolver::default();
      let suffixes = resolver.get_suffixes("fr");

      assert!(suffixes.is_ok());
      let suffixes = suffixes.unwrap();

      println!("{suffixes:?}");
      assert_eq!(suffixes.len(), 3);
      assert_eq!(suffixes, vec!["_one", "_many", "_other"]);
    }

    #[test_log::test]
    fn get_suffixes_return_elements_for_nl() {
      let resolver = PluralResolver::default();
      let suffixes = resolver.get_suffixes("nl");

      assert!(suffixes.is_ok());
      let suffixes = suffixes.unwrap();

      println!("{suffixes:?}");
      assert_eq!(suffixes.len(), 2);
      assert_eq!(suffixes, vec!["_one", "_other"]);
    }

    #[test_log::test]
    fn get_suffixes_returns_empty_vector_for_non_existent_code() {
      let resolver = PluralResolver::default();
      let suffixes = resolver.get_suffixes("nonexistent");
      assert!(suffixes.is_err());
    }

    #[test_log::test]
    fn get_suffix_returns_empty_string_for_non_existent_code() {
      let resolver = PluralResolver::default();
      assert_eq!(resolver.get_suffix("nonexistent", 1), "");
    }
  }
}
