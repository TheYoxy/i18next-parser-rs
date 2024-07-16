//! Macros for the core crate.

/// Log the execution time of a function.
#[macro_export]
macro_rules! log_time {
  ($message:expr, $func:expr) => {{
    use std::time::Instant;

    use color_eyre::owo_colors::{AnsiColors, OwoColorize};
    use log::trace;

    let start = Instant::now();
    let result = $func;
    let duration = start.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;
    let duration_str = if duration_ms < 10.0 {
      duration_ms.color(AnsiColors::Cyan)
    } else if duration_ms < 50.0 {
      duration_ms.color(AnsiColors::Yellow)
    } else {
      duration_ms.color(AnsiColors::Red)
    };
    trace!("{} - Execution time: {:.2} ms", $message, duration_str);
    result
  }};
}
