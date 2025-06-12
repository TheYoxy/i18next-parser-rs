use std::path::PathBuf;

use crate::visitor::traits::oxc_program::OxcProgram;

#[allow(unused_variables)]
pub fn print_error_location_from_file<Path>(file_name: Path, start: usize, end: usize)
where
  Path: Into<PathBuf>,
{
  #[cfg(feature = "print_error_location")]
  {
    use std::fs;

    let file_path: PathBuf = file_name.into();
    _ = fs::read_to_string(&file_path).and_then(|content| {
      print_error_location(&content, start, end);
      Ok(())
    });
  }
}

#[cfg(feature = "print_error_location")]
pub fn print_error_location(content: &str, start: usize, end: usize) {
  {
    use bat::{
      Input,
      PrettyPrinter,
      line_range::{LineRange, LineRanges},
    };
    use color_eyre::owo_colors::OwoColorize;

    #[inline]
    fn get_line_bounds(text: &str, start_char_index: usize, end_char_index: usize) -> Option<(usize, usize)> {
      if start_char_index > end_char_index || end_char_index > text.len() {
        return None; // Invalid indices
      }

      let mut current_line = 0; // 0-indexed line numbers

      let mut start_line: Option<usize> = None;
      let mut end_line: Option<usize> = None;

      // Iterate through characters and find the line bounds
      for (idx, c) in text.char_indices() {
        if idx >= start_char_index && start_line.is_none() {
          start_line = Some(current_line);
        }

        if idx >= end_char_index && end_line.is_none() {
          end_line = Some(current_line);
          // If we found both, we can break early
          if start_line.is_some() && end_line.is_some() {
            break;
          }
        }

        if c == '\n' {
          current_line += 1;
        }

        // If we've already passed the end_char_index by a significant margin
        // and haven't found end_line, it implies the end_char_index is within
        // the last line if the string doesn't end with a newline.
        // This break is an optimization.
        if end_line.is_some() && idx > end_char_index + 10 {
          // +10 is arbitrary for some buffer
          break;
        }
      }

      // Handle cases where the end_char_index is at the very end of the string
      // and there's no trailing newline, or if it's past the last newline.
      if let Some(s_line) = start_line {
        if let Some(e_line) = end_line {
          Some((s_line, e_line))
        } else {
          // If end_line was not set, it means the end_char_index
          // is within the very last line of the string.
          Some((s_line, current_line))
        }
      } else {
        // This case should ideally not happen if start_char_index is valid,
        // but included for robustness.
        None
      }
    }

    let bounds = get_line_bounds(content, start, end);
    let (start_line, end_line) = match bounds {
      Some((start, end)) => (start + 1, end + 1), // Convert to 1-indexed lines
      None => {
        log::error!("{} Invalid span: {start} {end}", "[Print_error_location]".red().bold());
        return;
      },
    };

    const BOUND: usize = 3;
    let range = LineRange::new(start_line.saturating_sub(BOUND), end_line + BOUND);
    let input = Input::from_bytes(content.as_bytes());
    let _ = PrettyPrinter::new()
      .input(input)
      .language("typescript")
      .line_ranges(LineRanges::from(vec![range]))
      .header(false)
      .grid(true)
      .line_numbers(true)
      .highlight_range(start_line, end_line)
      .print();
  }
}

pub trait PrintErrorLocation: OxcProgram {
  /// Print the location of an error in the source code
  #[tracing::instrument(skip(self), target = "instrument")]
  fn print_error_location(&self, _span: &oxc_span::Span) {
    #[cfg(feature = "print_error_location")]
    {
      let span = _span;
      let content = self.program().source_text;

      let start_pos = usize::try_from(span.start).unwrap();
      let end_pos = usize::try_from(span.end).unwrap();
      print_error_location(content, start_pos, end_pos);
    }
  }
}
