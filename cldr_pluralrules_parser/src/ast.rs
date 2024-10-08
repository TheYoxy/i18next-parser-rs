#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
  pub condition: Condition,
  pub samples: Option<Samples>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Samples {
  pub integer: Option<SampleList>,
  pub decimal: Option<SampleList>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleList {
  pub sample_ranges: Vec<SampleRange>,
  pub ellipsis: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleRange {
  pub lower_val: DecimalValue,
  pub upper_val: Option<DecimalValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecimalValue {
  pub integer: Value,
  pub decimal: Option<Value>,
}

/// A complete (and the only complete) AST representation of a plural rule. Comprises a vector of AndConditions.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "i = 5"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// Condition(vec![AndCondition(vec![Relation {
///   expression: Expression { operand: Operand::I, modulus: None },
///   operator: Operator::EQ,
///   range_list: RangeList(vec![RangeListItem::Value(Value(5))]),
/// }])]);
/// ```
///
/// Because they care complete representations, hand-written Conditions can be verified with the assert macro. No other AST nodes can be verified.
///
/// ```
/// use cldr_pluralrules_parser::{ast::*, parse_plural_rule};
///
/// let condition = Condition(vec![
///   AndCondition(vec![Relation {
///     expression: Expression { operand: Operand::I, modulus: None },
///     operator: Operator::Is,
///     range_list: RangeList(vec![RangeListItem::Value(Value(5))]),
///   }]),
///   AndCondition(vec![Relation {
///     expression: Expression { operand: Operand::V, modulus: None },
///     operator: Operator::Within,
///     range_list: RangeList(vec![RangeListItem::Value(Value(2))]),
///   }]),
/// ]);
///
/// assert_eq!(
///   condition,
///   parse_plural_rule("i is 5 or v within 2").expect("Parsing succeeded").condition
/// )
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Condition(pub Vec<AndCondition>);

/// An incomplete AST representation of a plural rule. Comprises a vector of Relations.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "i = 3 and v = 0"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// AndCondition(vec![
///   Relation {
///     expression: Expression { operand: Operand::I, modulus: None },
///     operator: Operator::In,
///     range_list: RangeList(vec![RangeListItem::Value(Value(5))]),
///   },
///   Relation {
///     expression: Expression { operand: Operand::V, modulus: None },
///     operator: Operator::NotIn,
///     range_list: RangeList(vec![RangeListItem::Value(Value(2))]),
///   },
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AndCondition(pub Vec<Relation>);

/// An incomplete AST representation of a plural rule. Comprises an Expression, an Operator, and a RangeList.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "i = 3"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// Relation {
///   expression: Expression { operand: Operand::I, modulus: None },
///   operator: Operator::Is,
///   range_list: RangeList(vec![RangeListItem::Value(Value(3))]),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
  pub expression: Expression,
  pub operator: Operator,
  pub range_list: RangeList,
}

/// An enum of Relation operators for plural rules.
///
/// Each Operator enumeration belongs to the corresponding symbolic operators:
///
/// | Enum Operator | Symbolic Operator |
/// | - | - |
/// | In | "in" |
/// | NotIn | "not in" |
/// | Within | "within" |
/// | NotWithin | "not within" |
/// | Is | "is" |
/// | IsNot | "is not" |
/// | EQ | "=" |
/// | NotEq | "!=" |
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
  In,
  NotIn,
  Within,
  NotWithin,
  Is,
  IsNot,
  EQ,
  NotEQ,
}

/// An incomplete AST representation of a plural rule. Comprises an Operand and an optional Modulo.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "i % 100"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// Expression { operand: Operand::I, modulus: Some(Modulo(Value(100))) };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
  pub operand: Operand,
  pub modulus: Option<Modulo>,
}

/// An incomplete AST representation of a plural rule. Comprises a Value but is later expressed as `% usize`.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "% 100"
/// ```
///
/// Will be used to represent the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// Modulo(Value(100));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Modulo(pub Value);

/// An incomplete AST representation of a plural rule. Comprises a char.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "i"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::Operand;
///
/// Operand::I;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
  C, // compact decimal exponent value: exponent of the power of 10 used in compact decimal formatting
  E, // deprecated synonym of C
  N, // Absolute value of input
  I, // Integer value of input
  V, // Number of visible fraction digits with trailing zeros
  W, // Number of visible fraction digits without trailing zeros
  F, // Visible fraction digits with trailing zeros
  T, // Visible fraction digits without trailing zeros
}
impl From<Operand> for &str {
  fn from(value: Operand) -> Self {
    match value {
      Operand::C => "c",
      Operand::E => "e",
      Operand::N => "n",
      Operand::I => "i",
      Operand::V => "v",
      Operand::T => "t",
      Operand::W => "w",
      Operand::F => "f",
    }
  }
}

/// An incomplete AST representation of a plural rule. Comprises a vector of RangeListItems.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "5, 7, 9"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// RangeList(vec![
///   RangeListItem::Value(Value(5)),
///   RangeListItem::Value(Value(7)),
///   RangeListItem::Value(Value(9)),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RangeList(pub Vec<RangeListItem>);

/// An enum of items that appear in a RangeList: Range or a Value.
///
/// See Range and Value for additional details.
#[derive(Debug, Clone, PartialEq)]
pub enum RangeListItem {
  Range(Range),
  Value(Value),
}

/// An incomplete AST representation of a plural rule. Comprises two Values: an inclusive lower and upper limit.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "11..15"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// RangeListItem::Range(Range { lower_val: Value(11), upper_val: Value(15) });
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
  pub lower_val: Value,
  pub upper_val: Value,
}

/// An incomplete AST representation of a plural rule, representing one integer.
///
/// # Examples
///
/// All AST nodes can be built explicitly, as seen in the example. However, due to its complexity, it is preferred to build the AST using the parse_plural_rule function.
///
/// ```text
/// "99"
/// ```
///
/// Can be represented by the AST:
///
/// ```
/// use cldr_pluralrules_parser::ast::*;
///
/// RangeListItem::Value(Value(99));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Value(pub usize);
