use color_eyre::eyre::Result;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use std::path::PathBuf;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

lazy_static! {
  pub(crate) static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
  pub(crate) static ref DATA_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_DATA", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  pub(crate) static ref CONFIG_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_CONFIG", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  pub(crate) static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
  pub(crate) static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

pub(crate) fn initialize_logging() -> Result<()> {
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

fn project_directory() -> Option<ProjectDirs> {
  ProjectDirs::from("be", "endevops", env!("CARGO_PKG_NAME"))
}

pub(crate) fn get_data_dir() -> PathBuf {
  let directory = if let Some(s) = DATA_FOLDER.clone() {
    s
  } else if let Some(proj_dirs) = project_directory() {
    proj_dirs.data_local_dir().to_path_buf()
  } else {
    PathBuf::from(".").join(".data")
  };
  directory
}
