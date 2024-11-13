/// ! Print the configuration.
use std::fmt::Display;

use color_eyre::owo_colors::OwoColorize;

use crate::config::Config;

fn print_info<Title: Display, Value: Display>(title: Title, value: Value) {
  eprintln!("  {} {}", title.bright_cyan(), value);
}

/// Print the configuration.
pub fn print_config<C: AsRef<Config>>(config: C) {
  let config = config.as_ref();
  let separator = "-------------------".bright_cyan();
  eprintln!("  {}", "i18next Parser rust".bright_cyan());
  eprintln!("  {}", separator);
  print_info("Dir:    ", config.working_dir.display());
  print_info("Input:  ", config.input.join(", "));
  print_info("Output: ", &config.output);
  if config.verbose {
    eprintln!("  {}", separator);
    print_info("Default value:       ", &config.default_value);
    print_info("Default namespace:   ", &config.default_namespace);
    eprintln!("  {}", separator);
    print_info("Context separator:   ", &config.context_separator);
    print_info("Namespace separator: ", &config.namespace_separator);
    print_info("Key separator:       ", &config.key_separator);
    print_info("Plural separator:    ", &config.plural_separator);
    eprintln!("  {}", separator);
    print_info("Locales:             ", config.locales.join(", "));
    eprintln!("  {}", separator);
  }
  eprintln!()
}
