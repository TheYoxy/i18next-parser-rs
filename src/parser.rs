use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref SINGLE_QUOTE_PATTERN: Regex = Regex::new("'(?:[^'].*?[^\\\\])?'").unwrap();
    static ref DOUBLE_QUOTE_PATTERN: Regex = Regex::new(r#"(?:[^"].*?[^\\\\])?"#).unwrap();
    static ref BACK_QUOTE_PATTERN: Regex = Regex::new(r#"`(?:[^`].*?[^\\\\])?`"#).unwrap();
    static ref VAR_PATTERN: Regex = Regex::new("(?:[A-Z0-9_.-]+)").unwrap();
}

fn parse_tree() {}

// pub fn aa(module: &Module) {
//     println!("");
//     module.body.iter().for_each(|stmt| match stmt {
//         ModuleItem::Stmt(expr) => {
//             println!("Statement: {:?}", expr);
//         }
//         ModuleItem::ModuleDecl(_) => todo!(),
//     });
//     println!("");
// }
