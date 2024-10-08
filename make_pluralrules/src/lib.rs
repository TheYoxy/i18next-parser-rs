//! make_pluralrules generates a Rust code representation of CLDR plural rules in compliance with [Unicode](https://unicode.org/reports/tr35/tr35-numbers.html#Language_Plural_Rules).
//!
//! Representations of plural rules are generated from [Unicode's plural rules](https://github.com/unicode-cldr/cldr-core/blob/master/supplemental/plurals.json) and uses the intl_pluralrules_parser AST to build the representation.
//!
//! The ouput is a Rust file, specified by the user in the comand
//! ```text
//! cargo run -- -i <./path/to/cldr.json>... -o <./path/to/output.rs>
//! ```

mod parser;

use std::collections::BTreeMap;

use color_eyre::eyre::{eyre, OptionExt};
use proc_macro2::TokenStream;
use unic_langid::LanguageIdentifier;

use crate::parser::{plural_category::PluralCategory, resource::*};

/// Takes a string representation of a CLDR JSON file and produces a string representation of the generated Rust code for the plural rules.
///
/// The string representation of the Rust code is written to a specified Rust file and can be used to get the plural category for numerical input.
pub fn generate_rs(cldr_jsons: &[String]) -> color_eyre::Result<String> {
  let mut cldr_version = None;
  let mut tokens = BTreeMap::new();

  for cldr_json in cldr_jsons {
    // resource_items is a struct representation of the raw CLDR rules.
    let resource_items = parse_plurals_resource_from_string(cldr_json)?;

    let res_cldr_version = resource_items.supplemental.version.cldr_version;

    if cldr_version.is_none() {
      cldr_version = Some(res_cldr_version);
    } else if cldr_version != Some(res_cldr_version) {
      return Err(eyre!("All input resources must use the same CLDR version!"));
    }

    if let Some(data) = resource_items.supplemental.plurals_type_cardinal {
      let rule_tokens = gen_type_rs(data)?;
      if tokens.contains_key("cardinal") {
        return Err(eyre!("Cannot provide two inputs with the same data!"));
      }
      tokens.insert("cardinal".to_owned(), rule_tokens);
    }

    if let Some(data) = resource_items.supplemental.plurals_type_ordinal {
      let rule_tokens = gen_type_rs(data)?;
      if tokens.contains_key("ordinal") {
        return Err(eyre!("Cannot provide two inputs with the same data!"));
      }
      tokens.insert("ordinal".to_owned(), rule_tokens);
    }
  }

  if cldr_version.is_none() || tokens.is_empty() {
    return Err(eyre!("None of the input files provided core data!"));
  }

  let cldr_version = &cldr_version.ok_or_eyre("No CLDR version found in input files!")?;
  // Call gen_rs to get Rust code. Convert TokenStream to string for file out.
  Ok(parser::gen_rs::gen_fn(tokens, cldr_version).to_string())
}

fn gen_type_rs(rules: BTreeMap<String, BTreeMap<String, String>>) -> color_eyre::Result<Vec<TokenStream>> {
  // rule_tokens is a vector of TokenStreams that represent the CLDR plural rules as Rust expressions.
  let mut rule_tokens = Vec::<TokenStream>::new();

  let mut rules: Vec<(LanguageIdentifier, BTreeMap<String, String>)> = rules
    .into_iter()
    .filter_map(|(key, value)| {
      if key == "root" || key == "und" {
        None
      } else {
        let langid = key.parse().ok()?;
        Some((langid, value))
      }
    })
    .collect();

  // We rely on sorted list for binary search in the consumer.
  rules.sort_unstable_by(|(langid1, _), (langid2, _)| langid1.cmp(langid2));

  for (lang, r) in rules {
    // this_lang_rules is a vector of plural rules saved as a PluralCategory and a TokenStream
    let mut this_lang_rules = Vec::<(PluralCategory, TokenStream)>::new();

    for (rule_name, rule_line) in r {
      // cat_name is the simplified category name from the CLDR source file
      let cat_name = rule_name.split('-').collect::<Vec<_>>()[2];

      // representation is the
      let representation =
        cldr_pluralrules_parser::parse_plural_condition(rule_line).expect("Parsing of a condition succeeded");
      let cat: PluralCategory = cat_name.into();

      // Only allow rules that are not `OTHER` to be added. `OTHER` can have no rules and is added outside the loop.
      if cat != PluralCategory::Other {
        let tokens = parser::gen_pr::gen_pr(representation);
        this_lang_rules.push((cat, tokens));
      }
    }
    // convert language rules to TokenStream and add them to all the rules
    rule_tokens.push(parser::gen_rs::gen_mid(&lang, &this_lang_rules)?);
  }

  Ok(rule_tokens)
}
