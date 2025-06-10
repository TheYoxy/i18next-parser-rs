use std::path::{Path, PathBuf};

use color_eyre::owo_colors::OwoColorize;
use log::{error, warn};
use oxc_allocator::Allocator;
use oxc_ast::ast::{IdentifierReference, ModuleDeclaration, Statement};
use oxc_parser::Parser;
use oxc_resolver::Resolver;
use oxc_span::SourceType;
use serde_json::Value;

use crate::visitor::traits::{oxc_custom_parser::OxcCustomParser, oxc_program::OxcProgram};

pub trait OxcResolver {
  fn file_path(&self) -> &PathBuf;
  fn resolver(&self) -> &Resolver;
  fn allocator(&self) -> &Allocator;
}

pub trait OxcResolveImport: OxcResolver + OxcProgram + OxcCustomParser {
  /// Parse a value from a module declaration
  fn parse_value_from_module_declaration(
    &self,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
    module: &ModuleDeclaration<'_>,
  ) -> Option<Value> {
    match module {
      ModuleDeclaration::ExportNamedDeclaration(decl) => {
        decl.declaration.as_ref().and_then(|declaration| {
          #[cfg(test)]
          log::trace!(
            "Parsing value from module declaration: {declaration:?} for {identifier:?}",
            declaration = declaration.bright_black().italic(),
            identifier = identifier.name.cyan()
          );
          let result = self.parse_value_for_identifier_from_declaration(identifier, declaration);
          #[cfg(test)]
          log::trace!("Result: {result:?}", result = result.yellow());
          result
        })
      },
      ModuleDeclaration::ImportDeclaration(import)
        if import
          .specifiers
          .as_ref()
          .is_some_and(|specifiers| specifiers.iter().any(|s| s.name().eq(&identifier.name))) =>
      {
        // let path = &self.options.root;
        let path =
          (if self.file_path().is_dir() { Some(self.file_path().as_path()) } else { self.file_path().parent() })
            .unwrap();

        let resolved = self.resolver().resolve(path, &import.source.value);
        match resolved {
          Ok(resolved) => self.find_value_identifier_and_declaration(resolved.path(), identifier),
          Err(err) => {
            error!("{} Failed to resolve import: {}", "[Parse_value_from_statement]".red().bold(), err);
            None
          },
        }
      },
      _ => {
        #[cfg(test)]
        warn!("Handle other module declarations: {module:?}");

        None
      },
    }
  }

  fn find_value_identifier_and_declaration(
    &self,
    path: &Path,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
  ) -> Option<Value> {
    let source_type = SourceType::from_path(path).unwrap();
    let source_text = std::fs::read_to_string(path).unwrap();
    let parser = Parser::new(self.allocator(), source_text.as_str(), source_type);
    let result = parser.parse();
    if result.panicked {
      warn!(
        "{} Failed to parse file {}: {:?}",
        "[Parse_i18next_option]".red().bold(),
        path.display().yellow(),
        result.errors
      );
      return None;
    }

    result.program.body.iter().find_map(|stmt| {
      #[cfg(test)]
      log::trace!(
        "Finding value for identifier from {} {identifier:?} in statement: {stmt:?}",
        path.display().yellow(),
        identifier = identifier.name.cyan(),
        stmt = stmt.bright_black().italic()
      );
      if let Some(module) = stmt.as_module_declaration() {
        let r = self.parse_value_from_module_declaration(identifier, module);
        #[cfg(test)]
        log::trace!("Found value: {r:?}", r = r.yellow());
        r
      } else {
        None
      }
    })
  }
}

impl<T> OxcCustomParser for T
where
  T: OxcResolveImport,
{
  fn find_value_for_identifier(
    &self,
    stmt: &Statement<'_>,
    identifier: &oxc_allocator::Box<'_, IdentifierReference<'_>>,
  ) -> Option<Value> {
    {
      if let Some(decl) = stmt.as_declaration() {
        #[cfg(test)]
        log::trace!(
          "Parsing declaration {:?} for identifier: {}",
          decl.bright_black().italic(),
          identifier.name.cyan()
        );
        self.parse_value_for_identifier_from_declaration(identifier, decl)
      } else if let Some(module) = stmt.as_module_declaration() {
        #[cfg(test)]
        log::trace!("Parsing module {:?} for identifier: {}", module.bright_black().italic(), identifier.name.cyan());
        self.parse_value_from_module_declaration(identifier, module)
      } else {
        #[cfg(test)]
        warn!("Unsupported statement: {stmt:?}");
        None
      }
    }
  }
}
