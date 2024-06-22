use color_eyre::owo_colors::OwoColorize;

use crate::config::Config;

pub(crate) fn print_config<C: AsRef<Config>>(config: C) {
  let config = config.as_ref();
  println!("  {}", "i18next Parser rust".bright_cyan());
  println!("  {}", "--------------".bright_cyan());
  let input = config.input.join(", ");
  println!("  {} {}", "Input: ".bright_cyan(), input);

  println!("  {} {}", "Output:".bright_cyan(), config.output);
  println!()
}
