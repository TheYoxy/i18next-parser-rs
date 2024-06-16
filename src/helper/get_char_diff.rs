pub(crate) fn get_char_diff(old: &str, new: &str) -> String {
  use color_eyre::owo_colors::OwoColorize;
  use itertools::Itertools;
  old
    .chars()
    .zip_longest(new.chars())
    .map(|pair| match pair {
      itertools::EitherOrBoth::Both(c1, c2) if c1 == c2 => c1.to_string(),
      itertools::EitherOrBoth::Both(c1, c2) => format!("{}{}", c1.to_string().red(), c2.to_string().green()),
      itertools::EitherOrBoth::Left(c1) => c1.to_string().red().to_string(),
      itertools::EitherOrBoth::Right(c2) => c2.to_string().green().to_string(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_print_diff_colored_same_strings() {
    let s1 = "Hello, world!";
    let s2 = "Hello, world!";
    let string = get_char_diff(s1, s2);
    println!("{string}");
    assert_eq!(string, "Hello, world!".to_string());
  }

  #[test]
  fn test_print_diff_colored_different_strings() {
    let s1 = "Hello, world!";
    let s2 = "Hello,  world!";
    let string = get_char_diff(s1, s2);
    println!("{string}");
    assert_eq!(
            string,
            "Hello, \u{1b}[31mw\u{1b}[39m\u{1b}[32m \u{1b}[39m\u{1b}[31mo\u{1b}[39m\u{1b}[32mw\u{1b}[39m\u{1b}[31mr\u{1b}[39m\u{1b}[32mo\u{1b}[39m\u{1b}[31ml\u{1b}[39m\u{1b}[32mr\u{1b}[39m\u{1b}[31md\u{1b}[39m\u{1b}[32ml\u{1b}[39m\u{1b}[31m!\u{1b}[39m\u{1b}[32md\u{1b}[39m\u{1b}[32m!\u{1b}[39m".to_string()
        );
  }

  #[test]
  fn test_print_diff_colored_different_lengths() {
    let s1 = "Hello, world!";
    let s2 = "Hello!";
    let string = get_char_diff(s1, s2);
    println!("{string}");
    assert_eq!(
            string,
            "Hello\u{1b}[31m,\u{1b}[39m\u{1b}[32m!\u{1b}[39m\u{1b}[31m \u{1b}[39m\u{1b}[31mw\u{1b}[39m\u{1b}[31mo\u{1b}[39m\u{1b}[31mr\u{1b}[39m\u{1b}[31ml\u{1b}[39m\u{1b}[31md\u{1b}[39m\u{1b}[31m!\u{1b}[39m".to_string()
        );
  }

  #[test]
  fn test_print_diff_colored_empty_strings() {
    let s1 = "";
    let s2 = "";
    let string = get_char_diff(s1, s2);
    println!("{string}");
    assert_eq!(string, "".to_string());
  }

  #[test]
  fn test_print_diff_colored_one_empty_string() {
    let s1 = "Hello, world!";
    let s2 = "";
    let string = get_char_diff(s1, s2);
    println!("{string}");
    assert_eq!(
            string,
            "\u{1b}[31mH\u{1b}[39m\u{1b}[31me\u{1b}[39m\u{1b}[31ml\u{1b}[39m\u{1b}[31ml\u{1b}[39m\u{1b}[31mo\u{1b}[39m\u{1b}[31m,\u{1b}[39m\u{1b}[31m \u{1b}[39m\u{1b}[31mw\u{1b}[39m\u{1b}[31mo\u{1b}[39m\u{1b}[31mr\u{1b}[39m\u{1b}[31ml\u{1b}[39m\u{1b}[31md\u{1b}[39m\u{1b}[31m!\u{1b}[39m".to_string()
        );
  }
}
