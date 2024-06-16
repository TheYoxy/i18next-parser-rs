#[macro_export]
macro_rules! printread {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let info_prefix = " [read] ".bright_green();
        println!("{}{}", info_prefix, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! printinfo {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let info_prefix = " [info] ".blue();
        println!("{}{}", info_prefix, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! printwarn {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let warn_prefix = " [warn] ".yellow();
        print!("{}{}", warn_prefix, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! printwarnln {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let warn_prefix = " [warn] ".yellow();
        println!("{}{}", warn_prefix, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! printerror {
    ($($arg:tt)*) => {{
        use color_eyre::owo_colors::OwoColorize;
        let err_prefix = " [err ] ".red();
        println!("{}{}", err_prefix, format!($($arg)*));
    }};
}
