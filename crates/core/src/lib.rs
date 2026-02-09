#![allow(dead_code)]
#![allow(clippy::large_enum_variant)]
// workaround https://github.com/rust-lang/rust-clippy/issues/13981
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod core;
pub mod inference;
pub mod path_utils;

pub use crate::core::*;
pub use beancount_parser::{Error, ParseError, Position, Result, ast};
pub use crate::inference::{
  infer_directives,
  infer_transaction_postings,
  InferredDirective,
  InferredPosting,
  InferredTransaction,
};
