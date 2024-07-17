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
#[cfg(test)]
mod log_time_tests {
  use std::{thread, time::Duration};

  /// Verifies that the log_time macro logs execution time for a fast function.
  #[test]
  fn logs_execution_time_for_fast_function() {
    let fast_function = || {
      thread::sleep(Duration::from_millis(5));
      42
    };
    let result = log_time!("fast_function", fast_function());
    assert_eq!(result, 42);
  }

  /// Verifies that the log_time macro logs execution time for a slow function.
  #[test]
  fn logs_execution_time_for_slow_function() {
    let slow_function = || {
      thread::sleep(Duration::from_millis(100));
      "slow"
    };
    let result = log_time!("slow_function", slow_function());
    assert_eq!(result, "slow");
  }

  /// Verifies that the log_time macro correctly handles a function that panics.
  #[test]
  #[should_panic(expected = "Intentional panic")]
  fn handles_panic_within_function() {
    let panic_function = || {
      panic!("Intentional panic");
    };
    log_time!("panic_function", panic_function());
  }
}
