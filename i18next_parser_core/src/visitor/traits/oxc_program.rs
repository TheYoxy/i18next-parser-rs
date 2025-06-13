use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_resolver::Resolver;

pub trait OxcProgram {
  fn program(&self) -> &Program<'_>;
  fn file_path(&self) -> &PathBuf;
  fn resolver(&self) -> &Resolver;
  fn allocator(&self) -> &Allocator;
  fn working_dir(&self) -> &PathBuf;
}
