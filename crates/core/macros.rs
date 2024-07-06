//! Macros for the core crate.

/// Log the execution time of a function.
#[macro_export]
macro_rules! log_time {
  ($message:expr, $func:expr) => {{
    use std::time::Instant;

    use log::debug;
    let start = Instant::now();
    let result = $func;
    let duration = start.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;
    debug!("{} - Execution time: {:.2} ms", $message, duration_ms);
    result
  }};
}

/// Print a message with a prefix of `[read]` in bright green color.
#[macro_export]
macro_rules! printread {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let info_prefix = " [read] ".bright_green();
        println!("{}{}", info_prefix, format!($($arg)*));
    }};
}

/// Print a message with a prefix of `[write]` in blue color.
#[macro_export]
macro_rules! printinfo {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let info_prefix = " [info] ".blue();
        println!("{}{}", info_prefix, format!($($arg)*));
    }};
}

/// Print a message with a prefix of `[warn]` in yellow color.
#[macro_export]
macro_rules! printwarn {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let warn_prefix = " [warn] ".yellow();
        print!("{}{}", warn_prefix, format!($($arg)*));
    }};
}

/// Print a message with a prefix of `[warn]` in yellow color.
#[macro_export]
macro_rules! printwarnln {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let warn_prefix = " [warn] ".yellow();
        println!("{}{}", warn_prefix, format!($($arg)*));
    }};
}

/// Print a message with a prefix of `[err ]` in red color.
#[macro_export]
macro_rules! printerror {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let err_prefix = " [err ] ".red();
        println!("{}{}", err_prefix, format!($($arg)*));
    }};
}
