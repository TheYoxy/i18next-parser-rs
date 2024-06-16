use log::info;
use std::fmt::Display;

pub(crate) fn log_execution_time<S, F, R>(message: S, func: F) -> R
where
  S: Display,
  F: FnOnce() -> R,
{
  use std::time::Instant;
  let start = Instant::now();
  let result = func();
  let duration = start.elapsed();
  let duration_ms = duration.as_secs_f64() * 1000.0;
  info!("{} - Execution time: {:.2} ms", message, duration_ms);
  result
}
