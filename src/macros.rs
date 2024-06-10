/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.

#[macro_export]
macro_rules! printwarn {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let warn_prefix = " WARN ".yellow();
        println!("{}{}", warn_prefix, format!($($arg)*));
    }};
}
