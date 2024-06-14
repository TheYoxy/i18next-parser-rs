use std::collections::HashMap;

fn get_cleaned_code(code: &str) -> String {
  if code.contains('_') {
    return code.replace('_', "-");
  }
  code.to_string()
}

struct PluralSet {
  lngs: Vec<&'static str>,
  nr: Vec<u32>,
  fc: u32,
}

fn create_rules(sets: Vec<PluralSet>, rules: &mut Rules) {
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
}

type RuleValue = (Vec<u32>, fn(u32) -> u32);
type Rules = HashMap<&'static str, RuleValue>;
pub struct PluralResolver {
  rules: Rules,
  simplify_plural_suffix: bool,
}

impl Default for PluralResolver {
  fn default() -> Self {
    Self::new(true)
  }
}

impl PluralResolver {
  pub fn new(simplify_plural_suffix: bool) -> Self {
    let mut rules = HashMap::new();
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
    create_rules(sets, &mut rules);

    Self { rules, simplify_plural_suffix }
  }

  fn get_rule(&self, code: &str) -> Option<&RuleValue> {
    let cleaned_code = get_cleaned_code(code);
    self.rules.get(cleaned_code.as_str())
  }

  pub fn get_suffixes(&self, code: &str) -> Vec<String> {
    match self.get_rule(code) {
      Some((numbers, _)) => numbers.iter().map(|&n| self.get_suffix(code, n)).collect(),
      None => vec![],
    }
  }

  fn get_suffix(&self, code: &str, count: u32) -> String {
    match self.get_rule(code) {
      Some((_, plural_func)) => {
        let idx = plural_func(count);
        if self.simplify_plural_suffix {
          match idx {
            1 => "".to_string(),
            2 => "plural".to_string(),
            _ => idx.to_string(),
          }
        } else {
          idx.to_string()
        }
      },
      None => String::new(),
    }
  }
}
