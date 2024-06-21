use color_eyre::owo_colors::OwoColorize;

use crate::config::Config;

pub(crate) fn print_config(config: &Config) {
    println!("  {}", "i18next Parser rust".bright_cyan());
    println!("  {}", "--------------".bright_cyan());
    let input = config.input.iter().map(|input| input.as_str()).collect::<Vec<_>>().join(", ");
    println!("  {} {}", "Input: ".bright_cyan(), input);

    println!("  {} {}", "Output:".bright_cyan(), config.output);
}
