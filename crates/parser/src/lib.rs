#![allow(dead_code)]
#![allow(clippy::large_enum_variant)]
// workaround https://github.com/rust-lang/rust-clippy/issues/13981
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod ast;
mod parser;
mod utils;

pub use parser::{parse_diagnostics, parse_lossy, parse_strict, render_strict_error};

#[deprecated(note = "use parse_lossy instead")]
pub fn parse_str(input: &str) -> Vec<ast::Directive<'_>> {
  parse_lossy(input)
}

use chumsky::error::Rich;
use chumsky::prelude::*;
use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
  pub line: usize,
  pub column: usize,
}

pub(crate) fn position_from_rope(rope: &Rope, offset: usize) -> Position {
  let char_idx = rope.byte_to_char(offset);
  let line_idx = rope.char_to_line(char_idx);
  let line_start_char = rope.line_to_char(line_idx);
  let line_start_byte = rope.char_to_byte(line_start_char);

  Position {
    line: line_idx + 1,
    column: offset.saturating_sub(line_start_byte) + 1,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
  pub line: usize,
  pub column: usize,
  pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseDiagnosticKind {
  Syntax,
  Semantic,
  State,
  Option,
  Recovery,
}

impl ParseDiagnosticKind {
  pub const fn as_str(self) -> &'static str {
    match self {
      ParseDiagnosticKind::Syntax => "syntax",
      ParseDiagnosticKind::Semantic => "semantic",
      ParseDiagnosticKind::State => "state",
      ParseDiagnosticKind::Option => "option",
      ParseDiagnosticKind::Recovery => "recovery",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnosticSpan {
  pub label: String,
  pub span: ast::Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
  pub kind: ParseDiagnosticKind,
  pub line: usize,
  pub column: usize,
  pub span: ast::Span,
  pub message: String,
  pub reason: String,
  pub contexts: Vec<ParseDiagnosticSpan>,
  pub related: Vec<ParseDiagnosticSpan>,
  pub annotation: Option<String>,
}

impl std::fmt::Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}: {}", self.line, self.column, self.message)
  }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

pub type Error<'src> = extra::Err<Rich<'src, char>>;

#[cfg(test)]
mod tests {
  use super::{ParseDiagnosticKind, ast, parse_diagnostics, parse_lossy};

  #[test]
  fn parses_crlf_input() {
    let src = [
      "2014-01-01 open Assets:Cash USD",
      "",
      "2014-01-02 * \"Lunch\"",
      "  Assets:Cash -10 USD",
      "  Expenses:Food 10 USD",
      "",
    ]
    .join("\r\n");

    let directives = parse_lossy(&src);

    assert_eq!(2, directives.len(), "{:?}", directives);
  }

  #[test]
  fn recovers_to_raw_after_error_line() {
    let src = [
      "2014-01-01 open Assets:Cash USD",      // 0
      "This is not valid",                    // 1
      "2014-01-02 balance Assets:Bank 1 USD", // 2
      "  b: TRUE",
      "2014-01-01 broken Assets:Cash USD", // 3 raw
      "2014-01-01 open Assets:Cash USD",   // 4
      "2014-01-01 open Assets:Cash USD",   // 5 raw
      "  broken meta",
      "2014-01-01 open Assets:Cash USD", // 6
      "",
    ]
    .join("\n");

    let directives = parse_lossy(&src);
    assert!(matches!(directives[0], ast::Directive::Open(_)));
    assert!(matches!(directives[1], ast::Directive::Raw(_)));
    assert!(matches!(directives[2], ast::Directive::Balance(_)));
    assert!(matches!(directives[3], ast::Directive::Raw(_)));
    assert!(matches!(directives[4], ast::Directive::Open(_)));
    assert!(matches!(directives[5], ast::Directive::Raw(_)));
    assert!(matches!(directives[6], ast::Directive::Open(_)));
    assert_eq!(directives.len(), 7);
  }

  #[test]
  fn parse_diagnostics_reports_structured_syntax_error() {
    let src = "This is not valid\n";

    let diagnostics = parse_diagnostics(src);

    assert_eq!(1, diagnostics.len(), "{diagnostics:?}");
    assert_eq!(ParseDiagnosticKind::Syntax, diagnostics[0].kind);
    assert_eq!("syntax error", diagnostics[0].reason);
    assert!(diagnostics[0].span.end >= diagnostics[0].span.start);
  }
}
