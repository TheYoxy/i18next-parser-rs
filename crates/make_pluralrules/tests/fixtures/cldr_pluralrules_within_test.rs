# ! [allow (unused_variables , unused_parens)] # ! [cfg_attr (feature = "cargo-clippy" , allow (clippy :: float_cmp))] # ! [cfg_attr (feature = "cargo-clippy" , allow (clippy :: unreadable_literal))] # ! [cfg_attr (feature = "cargo-clippy" , allow (clippy :: nonminimal_bool))] use super :: operands :: PluralOperands ; use super :: PluralCategory ; use unic_langid :: LanguageIdentifier ; use unic_langid :: subtags ; pub type PluralRule = fn (& PluralOperands) -> PluralCategory ; pub static CLDR_VERSION : usize = 0 ; macro_rules ! langid { ($ lang : expr , $ script : expr , $ region : expr) => { { unsafe { LanguageIdentifier :: from_raw_parts_unchecked ($ lang , $ script , $ region , None ,) } } } ; } pub const PRS_CARDINAL : & [(LanguageIdentifier , PluralRule)] = & [(langid ! (None , None , None) , | po | { if (2 . 0 <= po . n && po . n <= 10 . 0 && 8 > po . i && po . i > 9) { PluralCategory :: FEW } else if (po . n == 1 . 0) { PluralCategory :: ONE } else if (1 <= po . i % 10 && po . i % 10 <= 2) { PluralCategory :: TWO } else { PluralCategory :: OTHER } })] ;