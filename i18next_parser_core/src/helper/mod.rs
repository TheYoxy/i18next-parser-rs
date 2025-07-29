//! This module contains helper functions that are used in multiple places in the codebase.
pub mod clean_multi_line_code;
pub mod dot_path_to_hash;
pub mod get_char_diff;
pub mod merge_hashes;
pub mod resolver_helper;
mod serde_helper;
pub use fs::MakeRelativePath;
pub use serde_helper::{SerdeHelper, SerdeVecHelper};
mod fs;
pub mod html_entities_replacer;
