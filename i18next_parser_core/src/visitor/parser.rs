use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_resolver::Resolver;

use crate::{
  helper::resolver_helper::ResolveFromTsConfig,
  visitor::traits::{
    GetLineBound,
    oxc_custom_parser::OxcCustomParser,
    oxc_program::OxcProgram,
    print_error_location::PrintErrorLocation,
  },
};

pub struct ModuleParser<'a> {
  pub(super) resolver: Resolver,
  /// the file name of the file being parsed
  pub file_path: PathBuf,
  working_dir: &'a PathBuf,
  /// the program to be parsed
  pub program: &'a Program<'a>,
  allocator: &'a Allocator,
}

impl<'a> ModuleParser<'a> {
  pub fn new(program: &'a Program, allocator: &'a Allocator, file_path: PathBuf, working_dir: &'a PathBuf) -> Self {
    ModuleParser {
      program,
      allocator,
      file_path: file_path.clone(),
      working_dir,
      resolver: Resolver::from_ts_config(file_path).unwrap_or_default(),
    }
  }
}

impl PrintErrorLocation for ModuleParser<'_> {
}
impl OxcProgram for ModuleParser<'_> {
  fn program(&self) -> &Program<'_> {
    self.program
  }

  fn file_path(&self) -> &PathBuf {
    &self.file_path
  }

  fn resolver(&self) -> &Resolver {
    &self.resolver
  }

  fn allocator(&self) -> &Allocator {
    self.allocator
  }

  fn working_dir(&self) -> &PathBuf {
    self.working_dir
  }
}

impl GetLineBound for ModuleParser<'_> {
}
impl OxcCustomParser for ModuleParser<'_> {
}
