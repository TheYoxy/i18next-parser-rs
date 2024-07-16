use std::{fs, io::Write, path::PathBuf, process::Command};

use clap::Parser;
use make_pluralrules::generate_rs;

/// Initialize the panic handler.
pub fn initialize_panic_handler() -> color_eyre::Result<()> {
  let hooks = color_eyre::config::HookBuilder::default()
    .panic_section(format!("This is a bug. Consider reporting it at {}", env!("CARGO_PKG_REPOSITORY")))
    .capture_span_trace_by_default(false)
    .display_location_section(false)
    .display_env_section(false);
  if cfg!(debug_assertions) {
    hooks.install()
  } else {
    let (panic_hook, eyre_hook) = hooks.into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
      if cfg!(debug_assertions) {
        // Better Panic stacktrace that is only enabled when debugging.
        better_panic::Settings::auto()
          .most_recent_first(false)
          .lineno_suffix(true)
          .verbosity(better_panic::Verbosity::Full)
          .create_panic_handler()(panic_info);

        let msg = format!("{}", panic_hook.panic_report(panic_info));
        eprintln!("Error: {}", strip_ansi_escapes::strip_str(msg));
      } else {
        use human_panic::{handle_dump, print_msg, Metadata};
        let meta = Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
          .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", "))
          .homepage(env!("CARGO_PKG_HOMEPAGE"));

        let file_path = handle_dump(&meta, panic_info);
        // prints human-panic message
        print_msg(file_path, &meta).expect("human-panic: printing error message to console failed");
        eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
      }

      std::process::exit(1);
    }));

    Ok(())
  }
}

/// Gnerates Rust code for CLDR plural rules.
#[derive(Parser, Debug)]
#[command(version, about, author)]
struct Cli {
  /// Input CLDR JSON plural rules files
  #[arg(short, long)]
  input: Vec<PathBuf>,
  /// Output RS file
  #[arg(short, long)]
  output: PathBuf,
  /// Do not run `rustfmt` on the output file
  #[arg(short, long, default_value = "false")]
  ugly: bool,
}

fn main() -> color_eyre::Result<()> {
  initialize_panic_handler()?;
  let cli = Cli::parse();
  let input_paths = &cli.input;

  let input_jsons =
    input_paths.iter().map(|path| fs::read_to_string(path).expect("file not found")).collect::<Vec<_>>();
  let complete_rs_code = generate_rs(&input_jsons)?;

  let output_path = &cli.output;
  let mut file = fs::File::create(output_path)?;
  file.write_all(complete_rs_code.as_bytes())?;

  if !cli.ugly {
    println!("Running cargo fmt on {output_path:?}");
    Command::new("rustfmt").args([output_path]).output().expect("Failed to format the output using `rustfmt`");
  }

  Ok(())
}
