//! # Clean Multi Line Code
//!
//! `clean_multi_line_code` is a function that takes a string reference as an input and returns a new String.

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
/// use i18next_parser_core::clean_multi_line_code;
/// let result = clean_multi_line_code("\n \rThis is a test\n \r");
/// assert_eq!(result, "This is a test");
/// ```
pub fn clean_multi_line_code(text: &str) -> String {
  let result = replace_leading_trailing_newlines(text, "");
  replace_internal_newlines(&result, " ")
}

/// Function 1: Replace leading and trailing newlines followed by whitespace.
fn replace_leading_trailing_newlines(input: &str, replacement: &str) -> String {
  let mut chars = input.chars().peekable();
  let mut result = String::new();

  if let Some(&c) = chars.peek() {
    if c == '\n' || c == '\r' {
      chars.next();

      // Skip leading newlines and whitespace
      while let Some(&c) = chars.peek() {
        if c == '\n' || c == '\r' || c.is_whitespace() {
          chars.next();
        } else {
          break;
        }
      }
    }
  }

  // Collect remaining characters
  for c in chars {
    result.push(c);
  }

  // Trim trailing newlines and whitespace and replace with the replacement text
  let mut first = true;
  result = result
    .trim_end_matches(|c: char| {
      if first {
        first = false;
        c == '\n' || c == '\r'
      } else {
        c == '\n' || c == '\r' || c.is_whitespace()
      }
    })
    .to_string();

  // Add replacement text at the beginning and end of the result
  format!("{}{}{}", replacement, result, replacement)
}

/// Function 2: Replace standalone newline characters followed by whitespace in the middle of the string.
fn replace_internal_newlines(input: &str, replacement: &str) -> String {
  let mut result = String::new();
  let mut chars = input.chars().peekable();

  while let Some(c) = chars.next() {
    if c == '\n' || c == '\r' {
      result.push_str(replacement);

      // Skip any whitespace following the newline character
      while let Some(&next_c) = chars.peek() {
        if next_c.is_whitespace() {
          chars.next();
        } else {
          break;
        }
      }
    } else {
      // Add non-newline characters directly to result
      result.push(c);
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

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
  fn parse_keep_end() {
    assert_eq!(clean_multi_line_code("Attempt "), "Attempt ");
  }

  #[test]
  fn parse_keep_start() {
    assert_eq!(clean_multi_line_code(" on 10"), " on 10");
  }

  #[test]
  fn parse_with_args() {
    let input = "Reset password {{attempt}}";
    let expected = "Reset password {{attempt}}";
    assert_eq!(clean_multi_line_code(input), expected);
  }

  #[test]
  fn handles_newline_only_string() {
    let input = "\n\n\n";
    let expected = "";
    assert_eq!(clean_multi_line_code(input), expected);
  }
}
