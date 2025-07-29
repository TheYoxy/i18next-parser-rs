use std::path::{Path, PathBuf};

pub trait MakeRelativePath {
  fn make_relative<'a, P: AsRef<Path>>(&self, base_folder: P) -> PathBuf;
}

impl MakeRelativePath for Path {
  fn make_relative<'a, P: AsRef<Path>>(&self, base_folder: P) -> PathBuf {
    let path = std::path::absolute(self).unwrap();
    let path = path.strip_prefix(base_folder.as_ref()).unwrap_or(&path);
    path.to_path_buf()
  }
}

impl MakeRelativePath for &Path {
  fn make_relative<'a, P: AsRef<Path>>(&self, base_folder: P) -> PathBuf {
    let path = std::path::absolute(self).unwrap();
    let path = path.strip_prefix(base_folder.as_ref()).unwrap_or(&path);
    path.to_path_buf()
  }
}

impl MakeRelativePath for PathBuf {
  fn make_relative<'a, P: AsRef<Path>>(&self, base_folder: P) -> PathBuf {
    let path = std::path::absolute(self).unwrap();
    let path = path.strip_prefix(base_folder.as_ref()).unwrap_or(&path);
    path.to_path_buf()
  }
}

impl MakeRelativePath for &PathBuf {
  fn make_relative<'a, P: AsRef<Path>>(&self, base_folder: P) -> PathBuf {
    let path = std::path::absolute(self).unwrap();
    let path = path.strip_prefix(base_folder.as_ref()).unwrap_or(&path);
    path.to_path_buf()
  }
}
