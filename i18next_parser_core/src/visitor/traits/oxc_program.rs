use oxc_ast::ast::Program;

pub trait OxcProgram {
  fn program(&self) -> &Program<'_>;
}
