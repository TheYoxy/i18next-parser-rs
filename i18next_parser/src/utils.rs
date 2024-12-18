//! Collection of utility functions and constants used throughout the project.

use color_eyre::{eyre::Context, owo_colors::OwoColorize};
use tracing::{Event, Level, Subscriber};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
  filter::filter_fn,
  fmt,
  fmt::{format::FmtSpan, FormatEvent, FormatFields},
  layer::SubscriberExt,
  registry::LookupSpan,
  util::SubscriberInitExt,
  EnvFilter,
  Layer,
};

struct VerboseFilter {
  verbose: bool,
}
fn verbose_filter(verbose: bool) -> VerboseFilter {
  VerboseFilter { verbose }
}
impl<S> tracing_subscriber::layer::Filter<S> for VerboseFilter {
  /// Returns `true` if this layer is interested in a span or event with the
  /// given [`Metadata`] in the current [`Context`], similarly to
  /// [`Subscriber::enabled`].
  ///
  /// If this returns `false`, the span or event will be disabled _for the
  /// wrapped [`Layer`]_. Unlike [`Layer::enabled`], the span or event will
  /// still be recorded if any _other_ layers choose to enable it. However,
  /// the layer [filtered] by this filter will skip recording that span or
  /// event.
  ///
  /// If all layers indicate that they do not wish to see this span or event,
  /// it will be disabled.
  ///
  /// [`metadata`]: tracing_core::Metadata
  /// [`Subscriber::enabled`]: tracing_core::Subscriber::enabled
  /// [filtered]: crate::filter::Filtered
  fn enabled(&self, _meta: &tracing::Metadata<'_>, _cx: &tracing_subscriber::layer::Context<'_, S>) -> bool {
    self.verbose
  }
}

struct UserInfoFormatter;
impl<S, N> FormatEvent<S, N> for UserInfoFormatter
where
  S: Subscriber + for<'a> LookupSpan<'a>,
  N: for<'a> FormatFields<'a> + 'static,
{
  fn format_event(
    &self,
    ctx: &fmt::FmtContext<'_, S, N>,
    mut writer: fmt::format::Writer,
    event: &Event<'_>,
  ) -> std::fmt::Result {
    // Based on https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.FormatEvent.html#examples
    // Without the unused parts
    let metadata = event.metadata();

    let target = metadata.target();
    if target == "file_read" {
      write!(writer, "{} ", " [read] ".bright_green())?;
    } else if target == "count" {
    } else {
      return Ok(());
    }

    ctx.field_format().format_fields(writer.by_ref(), event)?;
    writeln!(writer)?;
    Ok(())
  }
}

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

    let target = metadata.target();
    if target == "file_read" || target == "count" {
      return Ok(());
    } else if target == "instrument_log" && cfg!(feature = "instrument") {
      write!(writer, "{} ", " [instr] ".bright_yellow())?;
    } else if level == Level::ERROR {
      write!(writer, "{} ", " [err ] ".red())?;
    } else if level == Level::WARN {
      write!(writer, "{} ", " [warn] ".yellow())?;
    } else if level == Level::INFO {
      write!(writer, "{} ", " [info] ".blue())?;
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

struct SpanFormatter;
impl<S, N> FormatEvent<S, N> for SpanFormatter
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
    write!(writer, " {}  ", "[time]".on_bright_cyan())?;

    ctx.field_format().format_fields(writer.by_ref(), event)?;

    let metadata = event.metadata();
    if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
      write!(writer, " @ {}:{}", file, line)?;
    }

    writeln!(writer)?;
    Ok(())
  }
}

pub fn initialize_logging(verbose: &bool) -> color_eyre::Result<()> {
  let file_subscriber = tracing_subscriber::fmt::layer()
    .compact()
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_writer(std::io::stderr)
    .with_target(false)
    .with_ansi(true)
    .event_format(InfoFormatter)
    .with_filter(EnvFilter::from_default_env())
    .with_filter(filter_fn(|meta| {
      let level = *meta.level();
      level <= Level::DEBUG
    }));

  let user_info_subscriber = tracing_subscriber::fmt::layer()
    .compact()
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_writer(std::io::stderr)
    .with_target(false)
    .with_ansi(true)
    .event_format(UserInfoFormatter)
    .with_filter(verbose_filter(*verbose));

  let span_subscriber = tracing_subscriber::fmt::layer()
    .compact()
    .with_writer(std::io::stderr)
    .with_file(true)
    .with_line_number(true)
    .with_target(true)
    .with_ansi(true)
    .with_span_events(FmtSpan::CLOSE)
    .event_format(SpanFormatter)
    .with_filter(filter_fn(|meta| {
      let target = meta.target();
      cfg!(feature = "instrument") && target == "instrument"
    }));

  let layer_trace = tracing_subscriber::fmt::layer()
    .with_writer(std::io::stderr)
    .without_time()
    .compact()
    .with_line_number(true)
    .with_filter(EnvFilter::from_default_env())
    .with_filter(filter_fn(|meta| *meta.level() > Level::DEBUG));

  tracing_subscriber::registry()
    .with(file_subscriber)
    .with(layer_trace)
    .with(span_subscriber)
    .with(user_info_subscriber)
    .with(ErrorLayer::default())
    .try_init()
    .with_context(|| "initializing logging")
}

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
        log::error!("Error: {}", strip_ansi_escapes::strip_str(msg));
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
