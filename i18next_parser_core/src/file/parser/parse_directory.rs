use std::{path::PathBuf, time::Instant};

use color_eyre::{
  eyre::bail,
  owo_colors::{CssColors, OwoColorize},
};
use log::debug;

use crate::{config::Config, file::parser::parse_file::parse_file, Entry};

/// Parse a directory and return a list of entries.
#[tracing::instrument(skip_all, err, target = "instrument")]
pub fn parse_directory<P: Into<PathBuf>, C: AsRef<Config>>(path: P, config: C) -> color_eyre::Result<Vec<Entry>> {
  let path = &path.into();
  if !path.exists() {
    bail!("Directory {path:?} does not exist");
  } else {
    debug!("Parsing directory {}", path.display().yellow());
  }

  let config = config.as_ref();
  debug!("Creating glob set from {:?}", &config.input.cyan());

  let glob = {
    let mut builder = globset::GlobSetBuilder::new();
    for input in &config.input {
      let join = path.join(input);
      let glob = join.to_str().unwrap();
      let glob = globset::GlobBuilder::new(glob)
        .literal_separator(true)
        .empty_alternates(false)
        .case_insensitive(true)
        .build()?;
      builder.add(glob);
    }
    builder.build()?
  };

  Ok(read_directory(path, config, &glob))
}

#[tracing::instrument(skip_all, target = "instrument")]
fn read_directory(path: &PathBuf, config: &Config, glob: &globset::GlobSet) -> Vec<Entry> {
  debug!("Reading directory {} to find {:?}", path.display().yellow(), &config.input.cyan());
  let values = ignore::WalkBuilder::new(path)
    .standard_filters(true)
    .build()
    .filter_map(Result::ok)
    .filter(|f| glob.is_match(f.path()))
    .filter_map(|entry| {
      let entry_path = entry.path();
      let now = Instant::now();
      let ret = parse_file(entry_path, config).ok();
      let elapsed = now.elapsed().as_secs_f64() * 1000.0;
      match &ret {
        Some(r) if !r.is_empty() => {
          let len = r.len();
          tracing::info!(target: "file_read", "{file} {format} {count}", file = entry_path.display(), count = format!("{len} translations").italic().color(CssColors::Gray), format = format!("({elapsed:.2}ms)").bright_black());
        },
        _ => {
          tracing::info!(target: "file_read", "{file} {format}", file = entry_path.display().italic().color(CssColors::Gray), format = format!("({elapsed:.2}ms)").bright_black());
        }
      }
      ret
    })
    .flatten().collect::<Vec<_>>();

  values
}
