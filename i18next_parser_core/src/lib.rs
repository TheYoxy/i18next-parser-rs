mod config;
mod file;
mod helper;
mod is_empty;
mod macros;
mod merger;
mod print;
mod transform;
mod visitor;

pub use config::Config;
pub use file::{parser::parse_directory::parse_directory, writer::write_to_file};
pub use helper::{clean_multi_line_code::clean_multi_line_code, merge_hashes::merge_hashes};
pub use is_empty::IsEmpty;
pub use merger::{merge_all_values::merge_all_values, merge_results::MergeResults};
pub use print::print_config::print_config;
pub use visitor::Entry;

#[cfg(feature = "generate_types")]
mod generate_types;

#[cfg(feature = "generate_types")]
pub use generate_types::generate_types;
