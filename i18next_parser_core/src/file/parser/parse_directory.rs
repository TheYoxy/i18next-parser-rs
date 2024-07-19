use std::{num::NonZero, path::PathBuf, time::Instant};

use color_eyre::{
  eyre::{bail, eyre},
  owo_colors::{CssColors, OwoColorize},
};
use ignore::DirEntry;
use log::{debug, info};
use tracing::instrument;

use crate::{config::Config, file::parser::parse_file::parse_file, log_time, Entry};

fn parse_directory_mono_thread(filter: &[DirEntry]) -> Vec<Entry> {
  filter
    .iter()
    .filter_map(move |entry| {
      let entry_path = entry.path();
      let now = Instant::now();
      let ret = parse_file(entry_path).ok();
      let elapsed = now.elapsed().as_secs_f64() * 1000.0;
      match &ret {
        Some(r) if !r.is_empty() => {
          let len = r.len();
            tracing::info!(target: "file_read", "{file} {format} {count}", file = entry_path.display(), count = format!("{len} translations").italic().color(CssColors::Gray) ,format = format!("({elapsed:.2}ms)").bright_black());
        },
        _ => {
            tracing::info!(target: "file_read", "{file} {format}", file = entry_path.display().italic().color(CssColors::Gray), format = format!("({elapsed:.2}ms)").bright_black());
        }
      }
      ret
    })
    .flatten()
    .collect()
}

fn parse_directory_thread(parallelism: NonZero<usize>, filter: &[DirEntry]) -> Vec<Entry> {
  let len = filter.len();
  let items_per_threads = len / parallelism;
  let chunk_size = (len + items_per_threads - 1) / items_per_threads; // ceil(len / n)

  let vectors = (0..items_per_threads)
    .map(|i| filter.iter().skip(i * chunk_size).take(chunk_size).cloned().collect::<Vec<_>>())
    .collect::<Vec<_>>();
  vectors
    .iter()
    .cloned()
    .flat_map(|filter| std::thread::spawn(move || parse_directory_mono_thread(&filter)).join().unwrap())
    .collect::<Vec<_>>()
}

/// Parse a directory and return a list of entries.
#[instrument(skip_all, err, target = "instrument")]
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
  log_time!(format!("Reading directory {}", directory_name.yellow()), {
    debug!("Reading directory {} to find {:?}", path.display().yellow(), &config.input);
    let filter = ignore::WalkBuilder::new(path)
      .standard_filters(true)
      .build()
      .filter_map(Result::ok)
      .filter(|f| glob.is_match(f.path()))
      .collect::<Vec<_>>();

    debug!("Found {} entries", filter.len().blue());
    if !filter.is_empty() {
      let parallelism = std::thread::available_parallelism().unwrap();
      let len = filter.len();

      info!("Reading {} files", len.blue());
      let entries = if len > parallelism.get() {
        debug!("Using {parallelism} threads to read the directory {directory_name}");
        parse_directory_thread(parallelism, &filter)
      } else {
        parse_directory_mono_thread(&filter)
      };

      Ok(entries)
    } else {
      bail!("No entries found in the directory {directory_name}")
    }
  })
}
