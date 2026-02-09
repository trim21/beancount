#![allow(dead_code)]
#![allow(clippy::large_enum_variant)]
// workaround https://github.com/rust-lang/rust-clippy/issues/13981
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod inference;

// Re-export parser-facing types so consumers can depend on `beancount-core` only.
pub use beancount_parser::core::{
  number_expr_to_decimal, Amount, CoreDirective, NumberExpr, Posting, Transaction,
  normalize_directives, normalize_directives_with_rope,
};
pub use beancount_parser::{Error, ParseError, Position, Result};
pub use crate::inference::{
  infer_directives,
  infer_transaction_postings,
  InferredDirective,
  InferredPosting,
  InferredTransaction,
};
