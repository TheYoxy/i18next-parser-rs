//! Collection of utility functions and constants used throughout the project.

use std::path::PathBuf;

use color_eyre::eyre::Result;
use lazy_static::lazy_static;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

lazy_static! {
  /// The name of the project.
  pub(crate) static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
  /// The folder where the data is stored.
  pub(crate) static ref DATA_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_DATA", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  /// The folder where the configuration is stored.
  pub(crate) static ref CONFIG_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_CONFIG", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  /// The log environment variable to check for the log level.
  pub(crate) static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
  /// The log file name.
  pub(crate) static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

#[cfg(debug_assertions)]
pub(crate) fn initialize_logging() -> color_eyre::Result<()> {
  use color_eyre::{eyre::Context, owo_colors::OwoColorize};
  use tracing::{Event, Level, Subscriber};
  use tracing_error::ErrorLayer;
  use tracing_subscriber::{
    filter::filter_fn,
    fmt,
    fmt::{FormatEvent, FormatFields},
    registry::LookupSpan,
    EnvFilter, Layer,
  };

  struct InfoFormatter;
  impl<S, N> FormatEvent<S, N> for InfoFormatter
  where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
  {
    fn format_event(
      &self,
      ctx: &fmt::FmtContext<'_, S, N>,
      mut writer: fmt::format::Writer,
      event: &Event,
    ) -> std::fmt::Result {
      // Based on https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.FormatEvent.html#examples
      // Without the unused parts
      let metadata = event.metadata();
      let level = *metadata.level();

      if level == Level::ERROR {
        write!(writer, "{} ", "!".red())?;
      } else if level == Level::WARN {
        write!(writer, "{} ", "!".yellow())?;
      } else if level == Level::INFO {
        write!(writer, "{} ", ">".green())?;
      } else {
        write!(writer, "{} ", "~".cyan())?;
      }

      ctx.field_format().format_fields(writer.by_ref(), event)?;

      if level != Level::INFO {
        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
          write!(writer, " @ {}:{}", file, line)?;
        }
      }

      writeln!(writer)?;
      Ok(())
    }
  }

  let file_subscriber = tracing_subscriber::fmt::layer()
    .compact()
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_writer(std::io::stderr)
    .with_target(false)
    .with_ansi(true)
    .event_format(InfoFormatter)
    .with_filter(filter_fn(|meta| {
      let level = *meta.level();
      level <= Level::DEBUG
    }))
    .with_filter(EnvFilter::from_default_env());

  let layer_debug = tracing_subscriber::fmt::layer()
    .with_writer(std::io::stderr)
    .without_time()
    .compact()
    .with_line_number(true)
    .with_filter(EnvFilter::from_default_env())
    .with_filter(filter_fn(|meta| *meta.level() > Level::DEBUG));

  tracing_subscriber::registry()
    .with(file_subscriber)
    .with(layer_debug)
    .with(ErrorLayer::default())
    .try_init()
    .with_context(|| "initializing logging")
}

/// Initialize the logging system.
#[cfg(not(debug_assertions))]
pub(crate) fn initialize_logging() -> Result<()> {
  use tracing_error::ErrorLayer;
  use tracing_subscriber::Layer;
  let directory = get_data_dir();
  std::fs::create_dir_all(&directory)?;
  let log_path = directory.join(LOG_FILE.clone());
  let log_file = std::fs::File::create(log_path)?;

  std::env::set_var(
    "RUST_LOG",
    std::env::var("RUST_LOG")
      .or_else(|_| std::env::var(LOG_ENV.clone()))
      .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME"))),
  );

  let file_subscriber = tracing_subscriber::fmt::layer()
    .with_file(true)
    .with_line_number(true)
    .with_writer(log_file)
    .with_target(false)
    .with_ansi(false)
    .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
  tracing_subscriber::registry().with(file_subscriber).with(ErrorLayer::default()).try_init()?;
  Ok(())
}

/// Initialize the panic handler.
pub(crate) fn initialize_panic_handler() -> Result<()> {
  let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
    .panic_section(format!("This is a bug. Consider reporting it at {}", env!("CARGO_PKG_REPOSITORY")))
    .capture_span_trace_by_default(false)
    .display_location_section(false)
    .display_env_section(false)
    .into_hooks();
  eyre_hook.install()?;
  std::panic::set_hook(Box::new(move |panic_info| {
    #[cfg(not(debug_assertions))]
    {
      use human_panic::{handle_dump, print_msg, Metadata};
      let meta = Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", "))
        .homepage(env!("CARGO_PKG_HOMEPAGE"));

      let file_path = handle_dump(&meta, panic_info);
      // prints human-panic message
      print_msg(file_path, &meta).expect("human-panic: printing error message to console failed");
      eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
    }
    let msg = format!("{}", panic_hook.panic_report(panic_info));
    log::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

    #[cfg(debug_assertions)]
    {
      // Better Panic stacktrace that is only enabled when debugging.
      better_panic::Settings::auto()
        .most_recent_first(false)
        .lineno_suffix(true)
        .verbosity(better_panic::Verbosity::Full)
        .create_panic_handler()(panic_info);
    }

    std::process::exit(libc::EXIT_FAILURE);
  }));
  Ok(())
}

/// Get the directory where the data is stored.
#[cfg(not(debug_assertions))]
pub(crate) fn get_data_dir() -> PathBuf {
  use directories::ProjectDirs;
  /// Get the directory where the project files are stored.
  fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("be", "endevops", env!("CARGO_PKG_NAME"))
  }

  let directory = if let Some(s) = DATA_FOLDER.clone() {
    s
  } else if let Some(proj_dirs) = project_directory() {
    proj_dirs.data_local_dir().to_path_buf()
  } else {
    PathBuf::from(".").join(".data")
  };
  directory
}
