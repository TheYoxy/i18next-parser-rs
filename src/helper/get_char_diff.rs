pub(crate) fn get_char_diff(old: &str, new: &str) -> String {
  use color_eyre::owo_colors::OwoColorize;
  use similar::{ChangeTag, TextDiff};

  TextDiff::from_chars(old, new)
    .iter_all_changes()
    .map(|changes| {
      let val = changes.value();
      match changes.tag() {
        ChangeTag::Equal => val.to_string(),
        ChangeTag::Insert => val.on_green().to_string(),
        ChangeTag::Delete => val.on_red().to_string(),
      }
    })
    .collect::<Vec<_>>()
    .concat()
}

#[cfg(test)]
mod tests {
  use super::*;
  use color_eyre::owo_colors::OwoColorize;

  #[test]
  fn get_char_diff_returns_empty_string_when_strings_are_identical() {
    let old = "Hello, World!";
    let new = "Hello, World!";
    let result = get_char_diff(old, new);
    assert_eq!(result, old);
  }

  #[test]
  fn get_char_diff_identifies_inserted_characters() {
    let old = "word";
    let new = "words";
    let result = get_char_diff(old, new);

    println!("{old} | {new} -> {result}");
    let format = format!("{old}{}", "s".on_green());
    println!("{result} == {format}");
    assert_eq!(result, format); // ANSI code for green
  }

  #[test]
  fn get_char_diff_identifies_deleted_characters() {
    let old = "words";
    let new = "word";
    let result = get_char_diff(old, new);

    println!("{old} | {new} -> {result}");
    let format = format!("word{}", "s".on_red());
    println!("{result} == {format}");
    assert_eq!(result, format); // ANSI code for red
  }

  #[test]
  fn get_char_diff_identifies_multiple_changes() {
    let old = "words";
    let new = "sword";
    let result = get_char_diff(old, new);

    println!("{old} | {new} -> {result}");
    let format = format!("{}word{}", "s".on_green(), "s".on_red());
    println!("{result} == {format}");
    assert_eq!(result, format); // ANSI codes for green and red
  }

  #[test]
  fn get_char_diff_handles_empty_strings() {
    let old = "";
    let new = "";
    assert_eq!(get_char_diff(old, new), "");
  }
}
