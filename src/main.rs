use std::path::Path;

use log::{debug, info, LevelFilter};
use oxc_allocator::Allocator;
use oxc_ast::{
    ast::{Expression, Program, Statement},
    visit::walk,
    Visit,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use simple_logger::SimpleLogger;

mod cli;
mod parser;

fn main() -> Result<(), String> {
    // let cli = Cli::parse();
    #[cfg(debug_assertions)]
    let level = LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let level = LevelFilter::Info;

    SimpleLogger::new().with_level(level).env().init().unwrap();

    let name = "tmp/file.tsx";
    let path = Path::new(&name);
    let source_text =
        std::fs::read_to_string(path).map_err(|_| format!("Unable to find {name}"))?;
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap();
    let ret = Parser::new(&allocator, &source_text, source_type).parse();

    for error in ret.errors {
        let error = error.with_source_code(source_text.clone());
        eprintln!("{error:?}");
    }

    let program = ret.program;

    let mut ast_pass = I18NVisitor::new(&program);

    info!("Start parsing...");
    let now = std::time::Instant::now();
    ast_pass.visit_program(&program);
    let elapsed_time = now.elapsed();
    info!("File parsed in {}ms.", elapsed_time.as_millis());

    // aa(&module);
    // println!("Module {:?}", module);

    Ok(())
}

#[derive(Debug)]
struct I18NVisitor<'a> {
    program: &'a Program<'a>,
    default_namespace: Option<String>,
}

impl<'a> I18NVisitor<'a> {
    /// Creates a new [`CountASTNodes`].
    fn new(program: &'a Program<'a>) -> Self {
        I18NVisitor { program }
    }

    /// Find the value of an identifier.
    fn find_identifier_value(
        &mut self,
        identifier: &oxc_allocator::Box<oxc_ast::ast::IdentifierReference>,
    ) -> Option<String> {
        let collect = &self
            .program
            .body
            .iter()
            .filter_map(|stmt| {
                if let Statement::VariableDeclaration(var) = stmt {
                    let filtered = var
                        .declarations
                        .iter()
                        .filter(|v| v.id.get_identifier() == Some(&identifier.name))
                        .collect::<Vec<_>>();
                    let item = filtered.first();
                    if let Some(item) = item {
                        if let Some(init) = &item.init {
                            parse_string_literal_or_satisfies_expression(init)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let arr = collect.first();

        arr.map(|arr| arr.to_string())
    }
    fn extract_namespace(&mut self, name: &str, expr: &oxc_ast::ast::CallExpression<'a>) {
        let arg = match name {
            "useTranslation" | "withTranslation" => expr.arguments.first(),
            "getFixedT" => expr.arguments.get(1),
            _ => None,
        };
        if let Some(arg) = arg {
            match arg {
                oxc_ast::ast::Argument::StringLiteral(str) => {
                    debug!("{name:?} Arg: {str:?}");
                    todo!("Handle string literal")
                }
                oxc_ast::ast::Argument::Identifier(identifier) => {
                    debug!("{name:?} Arg: {identifier:?}");
                    let identifier = self.find_identifier_value(identifier);
                    self.default_namespace = identifier;
                }
                _ => {}
            }
        }
    }
}

impl<'a> Visit<'a> for I18NVisitor<'a> {
    fn visit_tagged_template_expression(
        &mut self,
        expr: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        debug!("Tagged template: {:?}", expr);
        walk::walk_tagged_template_expression(self, expr);
    }

    fn visit_call_expression(&mut self, expr: &oxc_ast::ast::CallExpression<'a>) {
        // println!("Call expression: {:?}", expr);
        if let Some(name) = expr.callee_name() {
            self.extract_namespace(name, expr);
        }
        walk::walk_call_expression(self, expr);
    }
}

fn parse_string_literal_or_satisfies_expression(
    expr: &oxc_ast::ast::Expression<'_>,
) -> Option<String> {
    match expr {
        Expression::StringLiteral(str) => Some(str.value.to_string()),
        Expression::TSSatisfiesExpression(expr) => {
            parse_string_literal_or_satisfies_expression(&expr.expression)
        }
        _ => None,
    }
}
