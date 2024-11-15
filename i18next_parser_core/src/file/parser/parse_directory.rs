use std::{path::PathBuf, time::Instant};

use color_eyre::{
  eyre::{bail, eyre},
  owo_colors::{CssColors, OwoColorize},
};
use log::debug;

use crate::{config::Config, file::parser::parse_file::parse_file, log_time, Entry};

/// Parse a directory and return a list of entries.
#[tracing::instrument(skip_all, err, target = "instrument")]
pub fn parse_directory<P: Into<PathBuf>, C: AsRef<Config>>(path: P, config: C) -> color_eyre::Result<Vec<Entry>> {
  let path = &path.into();
  let config = config.as_ref();
  debug!("Creating globset from {:?}", &config.input);

  let glob = {
    let mut builder = globset::GlobSetBuilder::new();
    for input in &config.input {
      let join = path.join(input);
      let glob = join.to_str().unwrap();
      builder.add(globset::Glob::new(glob)?);
    }
    builder.build()?
  };

  if path.exists() {
    debug!("Reading directory {} to find {:?}", path.display().yellow(), &config.input);
  } else {
    bail!("Directory {path:?} does not exist");
  }

  for path in path.read_dir()? {
    let Ok(path) = path else { continue };
    debug!("Reading directory {} to find {:?}", path.path().display().yellow(), &config.input);
  }

  let directory_name =
    path.file_name().and_then(|s| s.to_str()).ok_or(eyre!("Unable to get filename of path {path:?}"))?;

  log_time!(format!("Reading directory {}", directory_name.yellow()), { Ok(read_directory(path, config, &glob)) })
}

fn read_directory(path: &PathBuf, config: &Config, glob: &globset::GlobSet) -> Vec<Entry> {
  debug!("Reading directory {} to find {:?}", path.display().yellow(), &config.input);
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
