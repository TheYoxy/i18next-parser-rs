/// `clean_multi_line_code` is a function that takes a string reference as an input and returns a new String.
/// It removes leading and trailing newline and whitespace characters from the input string.
/// It also replaces newline characters in the middle of the string with a space.
///
/// # Arguments
///
/// * `text` - A string slice that holds the text to be cleaned.
///
/// # Returns
///
/// * A String that represents the cleaned text.
///
/// # Examples
///
/// ```
/// let result = clean_multi_line_code("\n \rThis is a test\n \r");
/// assert_eq!(result, "This is a test");
/// ```
pub(crate) fn clean_multi_line_code(text: &str) -> String {
  use regex::Regex;

  let re_start_end = Regex::new(r"(^(\n|\r)\s*)|((\n|\r)\s*$)").unwrap();
  let re_middle = Regex::new(r"(\n|\r)\s*").unwrap();

  let result = re_start_end.replace_all(text, "");
  let result = re_middle.replace_all(&result, " ");

  result.into_owned()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn removes_leading_and_trailing_newlines_and_spaces() {
    let input = "\n \rThis is a test\n \r";
    let expected = "This is a test";
    assert_eq!(clean_multi_line_code(input), expected);
  }

  #[test]
  fn replaces_newlines_with_spaces() {
    let input = "This\nis\na\ntest";
    let expected = "This is a test";
    assert_eq!(clean_multi_line_code(input), expected);
  }

  #[test]
  fn preserves_spaces_between_words() {
    let input = "This    is    a    test";
    let expected = "This    is    a    test";
    assert_eq!(clean_multi_line_code(input), expected);
  }

  #[test]
  fn handles_empty_string() {
    let input = "";
    let expected = "";
    assert_eq!(clean_multi_line_code(input), expected);
  }

  #[test]
  fn handles_newline_only_string() {
    let input = "\n\n\n";
    let expected = "";
    assert_eq!(clean_multi_line_code(input), expected);
  }
}
