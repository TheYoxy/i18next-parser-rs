use std::{num::NonZero, path::PathBuf};

use color_eyre::eyre::{eyre, OptionExt};
use ignore::DirEntry;
use log::info;

use crate::{config::Config, log_time, parser::parse_file::parse_file, printinfo, visitor::Entry};

fn parse_directory_mono_thread(filter: &[DirEntry], is_verbose: bool) -> Vec<Entry> {
  filter
    .iter()
    .filter_map(move |entry| {
      let entry_path = entry.path();
      if is_verbose {
        crate::printread!("{}", entry_path.display());
      }
      parse_file(entry_path).ok()
    })
    .flatten()
    .collect()
}

fn parse_directory_thread(parallelism: NonZero<usize>, filter: &[DirEntry], is_verbose: bool) -> Vec<Entry> {
  let len = filter.len();
  let items_per_threads = len / parallelism;
  let chunk_size = (len + items_per_threads - 1) / items_per_threads; // ceil(len / n)

  let vectors = (0..items_per_threads)
    .map(|i| filter.iter().skip(i * chunk_size).take(chunk_size).cloned().collect::<Vec<_>>())
    .collect::<Vec<_>>();
  vectors
    .iter()
    .cloned()
    .flat_map(|filter| std::thread::spawn(move || parse_directory_mono_thread(&filter, is_verbose)).join().unwrap())
    .collect::<Vec<_>>()
}

/// Parse a directory and return a list of entries.
pub(crate) fn parse_directory<C: AsRef<Config>>(path: &PathBuf, config: C) -> color_eyre::Result<Vec<Entry>> {
  let config = config.as_ref();
  let inputs = &config.input;
  let mut builder = globset::GlobSetBuilder::new();
  for input in inputs {
    let join = path.join(input);
    let glob = join.to_str().unwrap();
    builder.add(globset::Glob::new(glob)?);
  }

  let glob = builder.build()?;

  let directory_name =
    path.as_path().file_name().and_then(|s| s.to_str()).ok_or_eyre("Unable to get filename of path {path:?}")?;

  log_time!(format!("Reading directory {directory_name}"), || {
    let filter = ignore::WalkBuilder::new(path)
      .standard_filters(true)
      .build()
      .filter_map(Result::ok)
      .filter(|f| glob.is_match(f.path()))
      .collect::<Vec<_>>();

    if filter.is_empty() {
      Err(eyre!("No entries found in the directory {directory_name}"))
    } else {
      let parallelism = std::thread::available_parallelism().unwrap();
      let len = filter.len();

      printinfo!("Reading {len} files");
      let is_verbose = config.verbose;
      let entries = if len > parallelism.get() {
        info!("Using {parallelism} threads to read the directory {directory_name}");
        parse_directory_thread(parallelism, &filter, is_verbose)
      } else {
        parse_directory_mono_thread(&filter, is_verbose)
      };

      Ok(entries)
    }
  })
}
