use std::{path::PathBuf, time::Instant};

use color_eyre::{
  eyre::{bail, eyre},
  owo_colors::{CssColors, OwoColorize},
};
use ignore::DirEntry;
use log::debug;
use tracing::instrument;

use crate::{Entry, config::Config, file::parser::parse_file::parse_file, log_time};

fn parse_directory_mono_thread<C: AsRef<Config>>(filter: &[DirEntry], config: C) -> Vec<Entry> {
  filter
    .iter()
    .filter_map(move |entry| {
      let entry_path = entry.path();
      let now = Instant::now();
      let ret = parse_file(entry_path, &config).ok();
      let elapsed = now.elapsed().as_secs_f64() * 1000.0;
      match &ret {
        Some(r) if !r.is_empty() => {
          let len = r.len();
          tracing::info!(target: "file_read", "{file} {format} {count}", file = entry_path.display(), format = format!("({elapsed:.2}ms)").bright_black(), count = format!("{len} translations").italic().color(CssColors::Gray) );
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

/// Chunks a slice into `num_chunks` approximately equal sub-slices.
///
/// Returns a `Vec` of slices, where each slice refers to a part of the original data.
/// This method is fast because it avoids copying data and only performs index calculations.
///
/// If `num_chunks` is 0, an empty Vec is returned.
/// If `num_chunks` is greater than the length of the slice, each element might become its own chunk,
/// or some chunks might be empty, depending on the distribution strategy.
///
/// # Arguments
/// * `data` - The slice to chunk.
/// * `num_chunks` - The desired number of sub-chunks.
#[cfg(feature = "multithreaded")]
pub fn chunk_into_n_sub_elements<T>(data: &[T], num_chunks: usize) -> Vec<&[T]> {
  let total_elements = data.len();

  if num_chunks == 0 {
    return Vec::new();
  }
  if total_elements == 0 {
    return vec![&[]; num_chunks]; // Return `num_chunks` empty slices if data is empty
  }
  if num_chunks >= total_elements {
    // If we want more chunks than elements, or equal,
    // make each element its own chunk, and add empty chunks if needed.
    let mut chunks: Vec<&[T]> = data.iter().map(|x| std::slice::from_ref(x)).collect();
    // Add empty slices if num_chunks is still greater than total_elements
    chunks.resize(num_chunks, &[]);
    return chunks;
  }

  let base_chunk_size = total_elements / num_chunks;
  let mut remainder = total_elements % num_chunks;

  let mut result_chunks: Vec<&[T]> = Vec::with_capacity(num_chunks);
  let mut current_idx = 0;

  for _i in 0..num_chunks {
    let mut current_chunk_size = base_chunk_size;
    if remainder > 0 {
      current_chunk_size += 1;
      remainder -= 1;
    }

    let end_idx = current_idx + current_chunk_size;
    // Ensure end_idx doesn't go out of bounds (edge case for last chunk if there's float arithmetic)
    let end_idx = end_idx.min(total_elements);

    result_chunks.push(&data[current_idx..end_idx]);
    current_idx = end_idx;
  }

  result_chunks
}

#[cfg(feature = "multithreaded")]
fn parse_directory_thread<'a>(parallelism: usize, filter: &'a [DirEntry], config: &'a Config) -> Vec<Entry> {
  let chunks = chunk_into_n_sub_elements(filter, parallelism);
  std::thread::scope(|scope| {
    let mut vec = Vec::<Entry>::new();
    for chunk in chunks {
      let val = scope.spawn(|| parse_directory_mono_thread(chunk, config)).join().unwrap();
      vec.extend(val);
    }
    vec
  })
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

  let exclude = {
    let mut builder = globset::GlobSetBuilder::new();
    for input in &config.exclude {
      let join = path.join(input);
      let glob = join.to_str().unwrap();
      builder.add(globset::Glob::new(glob)?);
    }
    builder.build()?
  };

  if !path.exists() {
    bail!("Directory {path:?} does not exist");
  }

  let directory_name =
    path.file_name().and_then(|s| s.to_str()).ok_or(eyre!("Unable to get filename of path {path:?}"))?;
  log_time!(format!("Reading directory {}", directory_name.yellow()), {
    debug!("Reading directory {} to find {:?}", path.display().yellow(), &config.input);

    let filter = ignore::WalkBuilder::new(path)
      .git_ignore(true)
      .git_global(true)
      .git_exclude(true)
      .hidden(false)
      .build()
      .filter_map(Result::ok)
      .filter(|f| glob.is_match(f.path()))
      .filter(|f| !exclude.is_match(f.path()))
      .collect::<Vec<_>>();
    #[cfg(feature = "multithreaded")]
    {
      let parallelism = std::thread::available_parallelism().unwrap();
      let len = filter.len();

      log::info!("Reading {} files", len.blue());
      let entries = if len > parallelism.get() {
        debug!("Using {parallelism} threads to read the directory {directory_name}");
        parse_directory_thread(parallelism.into(), &filter, config)
      } else {
        parse_directory_mono_thread(&filter, config)
      };

      Ok(entries)
    }
    #[cfg(not(feature = "multithreaded"))]
    {
      Ok(parse_directory_mono_thread(&filter, config))
    }
  })
}
