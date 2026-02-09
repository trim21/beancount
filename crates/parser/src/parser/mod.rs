use chumsky::prelude::*;
use ropey::Rope;

use crate::Error;
use crate::{ParseError, ast, position_from_rope};

#[cfg(feature = "rich-errors")]
type RawStrictError<'src> = chumsky::error::Rich<'src, char>;
#[cfg(not(feature = "rich-errors"))]
type RawStrictError<'src> = chumsky::error::Simple<'src, char>;

mod balance;
mod close;
mod comment;
mod commodity;
mod common;
mod custom;
mod document;
mod event;
mod headline;
mod include;
mod note;
mod number;
mod open;
mod option;
mod pad;
mod plugin;
mod popmeta;
mod poptag;
mod price;
mod pushmeta;
mod pushtag;
mod query;
mod raw;
mod transaction;

pub type StrictError = crate::ParseError;

fn skipped_line_parser<'src>()
-> impl Parser<'src, &'src str, Option<ast::Directive<'src>>, Error<'src>> {
  choice((
    common::ws0_parser().then_ignore(common::eol()).to(None),
    common::ws1_parser().then_ignore(end()).to(None),
  ))
}

#[cfg(feature = "rich-errors")]
fn describe_pattern(expected: chumsky::error::RichPattern<'_, char>) -> String {
  use chumsky::error::RichPattern;
  use chumsky::util::MaybeRef;

  match expected {
    RichPattern::Token(tok) => match tok {
      MaybeRef::Ref(ch) => describe_char(*ch),
      MaybeRef::Val(ch) => describe_char(ch),
    },
    RichPattern::Label(label) => label.to_string(),
    RichPattern::Identifier(ident) => ident.clone(),
    RichPattern::Any => "any character".to_string(),
    RichPattern::SomethingElse => "something else".to_string(),
    RichPattern::EndOfInput => "end of input".to_string(),
    _ => "input".to_string(),
  }
}

fn describe_char(ch: char) -> String {
  match ch {
    '\n' => "newline".to_string(),
    '\r' => "carriage return".to_string(),
    '\t' => "tab".to_string(),
    ' ' => "space".to_string(),
    other => format!("'{other}'"),
  }
}

fn describe_found(found: Option<&char>) -> String {
  found
    .map(|ch| describe_char(*ch))
    .unwrap_or_else(|| "end of input".to_string())
}

fn join_list(items: &[String]) -> String {
  match items {
    [] => String::new(),
    [one] => one.clone(),
    [first, second] => format!("{first} or {second}"),
    many => {
      let (rest, last) = many.split_at(many.len() - 1);
      format!("{}, or {}", rest.join(", "), last[0])
    }
  }
}

#[cfg(feature = "rich-errors")]
fn summarize_expected<'a>(
  expected: impl IntoIterator<Item = chumsky::error::RichPattern<'a, char>>,
) -> Option<String> {
  let mut items: Vec<String> = expected.into_iter().map(describe_pattern).collect();
  items.sort();
  items.dedup();

  if items.is_empty() {
    return None;
  }

  const MAX_EXPECTED: usize = 5;
  if items.len() <= MAX_EXPECTED {
    Some(join_list(&items))
  } else {
    let remaining = items.len() - MAX_EXPECTED;
    let head = join_list(&items[..MAX_EXPECTED]);
    Some(format!("{head}, and {remaining} more"))
  }
}

#[cfg(feature = "rich-errors")]
fn format_strict_reason(reason: &chumsky::error::RichReason<'_, char>) -> String {
  use chumsky::error::RichReason;

  match reason {
    RichReason::ExpectedFound { expected, found } => {
      let expected = summarize_expected(expected.clone());
      let found = describe_found(found.as_deref());
      match expected {
        Some(exp) => format!("expected {exp}, found {found}"),
        None => format!("unexpected {found}"),
      }
    }
    RichReason::Custom(msg) => msg.clone(),
  }
}

#[cfg(feature = "rich-errors")]
fn strict_error_message(err: &RawStrictError<'_>) -> String {
  format_strict_reason(err.reason())
}

#[cfg(not(feature = "rich-errors"))]
fn strict_error_message(err: &RawStrictError<'_>) -> String {
  let found = describe_found(err.found());
  format!("unexpected {found}")
}

fn strict_error_to_parse_error(source: &Rope, err: RawStrictError<'_>) -> ParseError {
  let span = err.span();
  let pos = position_from_rope(source, span.start);
  let message = strict_error_message(&err);

  ParseError {
    line: pos.line,
    column: pos.column,
    message,
  }
}

fn directive_parser<'src>()
-> impl Parser<'src, &'src str, ast::Directive<'src>, Error<'src>> + 'src {
  choice((
    include::include_directive_parser().then_ignore(common::line_end()),
    plugin::plugin_directive_parser().then_ignore(common::line_end()),
    option::option_directive_parser().then_ignore(common::line_end()),
    pushtag::pushtag_directive_parser().then_ignore(common::line_end()),
    poptag::poptag_directive_parser().then_ignore(common::line_end()),
    pushmeta::pushmeta_directive_parser().then_ignore(common::line_end()),
    popmeta::popmeta_directive_parser().then_ignore(common::line_end()),
    comment::comment_directive_parser().then_ignore(common::line_end()),
    headline::headline_directive_parser().then_ignore(common::line_end()),
    open::open_directive_parser(),
    close::close_directive_parser(),
    balance::balance_directive_parser(),
    pad::pad_directive_parser(),
    commodity::commodity_directive_parser(),
    price::price_directive_parser(),
    event::event_directive_parser(),
    query::query_directive_parser(),
    note::note_directive_parser(),
    document::document_directive_parser(),
    custom::custom_directive_parser(),
    transaction::transaction_directive_parser(),
  ))
  .recover_with(via_parser(common::raw_directive_recovery_parser()))
}

fn directive_parser_strict<'src>()
-> impl Parser<'src, &'src str, ast::Directive<'src>, Error<'src>> + 'src {
  choice((
    include::include_directive_parser().then_ignore(common::line_end()),
    plugin::plugin_directive_parser().then_ignore(common::line_end()),
    option::option_directive_parser().then_ignore(common::line_end()),
    pushtag::pushtag_directive_parser().then_ignore(common::line_end()),
    poptag::poptag_directive_parser().then_ignore(common::line_end()),
    pushmeta::pushmeta_directive_parser().then_ignore(common::line_end()),
    popmeta::popmeta_directive_parser().then_ignore(common::line_end()),
    comment::comment_directive_parser().then_ignore(common::line_end()),
    headline::headline_directive_parser().then_ignore(common::line_end()),
    open::open_directive_parser(),
    close::close_directive_parser(),
    balance::balance_directive_parser(),
    pad::pad_directive_parser(),
    commodity::commodity_directive_parser(),
    price::price_directive_parser(),
    event::event_directive_parser(),
    query::query_directive_parser(),
    note::note_directive_parser(),
    document::document_directive_parser(),
    custom::custom_directive_parser(),
    transaction::transaction_directive_parser(),
  ))
}

fn declarations_parser<'src>()
-> impl Parser<'src, &'src str, Vec<Option<ast::Directive<'src>>>, Error<'src>> + 'src {
  choice((skipped_line_parser(), directive_parser().map(Some)))
    .repeated()
    .collect::<Vec<_>>()
}

fn declarations_parser_strict<'src>()
-> impl Parser<'src, &'src str, Vec<Option<ast::Directive<'src>>>, Error<'src>> + 'src {
  choice((skipped_line_parser(), directive_parser_strict().map(Some)))
    .repeated()
    .collect::<Vec<_>>()
}

pub fn parse_lossy<'a>(source: &'a str) -> Vec<ast::Directive<'a>> {
  declarations_parser()
    .then_ignore(end())
    .parse(source)
    .into_output()
    .map(|directives| directives.into_iter().flatten().collect())
    .unwrap_or_else(Vec::new)
}

/// Parse without recovery to `Raw`; returns errors instead.
pub fn parse_strict<'a>(
  source: &'a str,
) -> Result<Vec<ast::Directive<'a>>, Vec<StrictError>> {
  let rope = Rope::from_str(source);

  declarations_parser_strict()
    .then_ignore(end())
    .parse(source)
    .into_result()
    .map(|directives| directives.into_iter().flatten().collect())
    .map_err(|errors| {
      errors
        .into_iter()
        .map(|err| strict_error_to_parse_error(&rope, err))
        .collect()
    })
}
