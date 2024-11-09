use crate::data::{self, Close, Custom, Metadata, Open, Plugin};
use crate::error::{ParseError, ParseResult};
use crate::parse::{Directive, File, Opt};
use crate::ParserError;
use chrono::NaiveDate;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::vec;

#[derive(Parser)]
#[grammar = "./grammar.pest"] // relative to src
pub struct MyParser;

#[derive(Debug, Default)]
struct ParseState<'i> {
    // Track pushed tag count with HashMap<&str, u64> instead of only tracking
    // tags with HashSet<&str> because the spec allows pushing multiple of the
    // same tag, and conformance with bean-check requires an equal number of
    // pops.
    pushed_tags: HashMap<&'i str, u16>,
}

impl<'i> ParseState<'i> {
    fn push_tag(&mut self, tag: &'i str) {
        *self.pushed_tags.entry(tag).or_insert(0) += 1;
    }

    fn pop_tag(&mut self, tag: &str) -> Result<(), String> {
        match self.pushed_tags.get_mut(tag) {
            Some(count) => {
                if *count <= 1 {
                    self.pushed_tags.remove(tag);
                } else {
                    *count -= 1;
                }
                Ok(())
            }
            _ => Err(format!("Attempting to pop absent tag: '{}'", tag)),
        }
    }

    fn get_pushed_tags(&self) -> impl Iterator<Item = &&str> {
        self.pushed_tags.keys()
    }
}

fn extract_tag<'i>(pair: Pair<'i, Rule>) -> ParseResult<&'i str> {
    let mut pairs = pair.into_inner();
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::invalid_state("tag"))?;
    Ok(&pair.as_str()[1..])
}

#[pyfunction(name = "parse2")]
pub fn py_parse2(content: &str) -> PyResult<File> {
    return parse2(content).or_else(|err| Err(ParserError::new_err(format!("{:#?}", err))));
}

pub fn parse2(content: &str) -> ParseResult<File> {
    let entries = MyParser::parse(Rule::file, content)?
        .next()
        .ok_or_else(|| ParseError::invalid_state("non-empty parse result"))?;

    let mut state = ParseState {
        // root_names: ["Assets", "Liabilities", "Equity", "Income", "Expenses"]
        //     .iter()
        //     .map(|ty| (ty.to_string(), ty.to_string()))
        //     .collect(),
        pushed_tags: HashMap::new(),
    };

    let mut directives = Vec::new();

    let mut includes = Vec::new();

    for entry in entries.into_inner() {
        match entry.as_rule() {
            Rule::EOI => {
                let pushed_tags = state
                    .get_pushed_tags()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<String>>()
                    .join(", ");
                if !pushed_tags.is_empty() {
                    return Err(ParseError::invalid_input_with_span(
                        format!("Unbalanced pushed tag(s): {}", pushed_tags),
                        entry.as_span(),
                    ));
                }
                break;
            }
            Rule::pushtag => {
                state.push_tag(extract_tag(entry)?);
            }
            Rule::poptag => {
                let span = entry.as_span();
                if let Err(msg) = state.pop_tag(extract_tag(entry)?) {
                    return Err(ParseError::invalid_input_with_span(msg, span));
                }
            }
            Rule::include => {
                includes.push(get_quoted_str(entry.into_inner().next().unwrap())?);
            }
            _ => {
                let dir = directive(entry, &state)?;

                directives.push(dir);
            }
        }
    }

    return Ok(File {
        includes,
        options: vec![],
        directives,
    });
}

fn directive(directive: Pair<Rule>, state: &ParseState) -> ParseResult<Directive> {
    let dir = match directive.as_rule() {
        Rule::option => option_directive(directive)?,
        Rule::plugin => plugin_directive(directive)?,
        Rule::custom => custom_directive(directive, state)?,
        Rule::open => open_directive(directive, state)?,
        Rule::close => close_directive(directive, state)?,
        // Rule::commodity_directive => commodity_directive(directive, state)?,
        // Rule::note => note_directive(directive, state)?,
        // Rule::pad => pad_directive(directive, state)?,
        // Rule::query => query_directive(directive, state)?,
        // Rule::event => event_directive(directive, state)?,
        // Rule::document => document_directive(directive, state)?,
        // Rule::price => price_directive(directive, state)?,
        // Rule::transaction => transaction_directive(directive, state)?,
        _ => Directive::S(format!("Unsupported {:#?}", directive).into()),
    };
    Ok(dir)
}

fn close_directive(directive: Pair<'_, Rule>, _state: &ParseState<'_>) -> ParseResult<Directive> {
    let mut pairs = directive.into_inner();

    Ok(Directive::Close(Close {
        date: date(pairs.next().unwrap())?,
        account: account(pairs.next().unwrap())?,
        meta: Metadata::new(),
    }))
}

fn option_directive<'i>(directive: Pair<'i, Rule>) -> ParseResult<Directive> {
    let mut paris = directive.into_inner();

    let name = get_quoted_str(paris.next().unwrap())?;
    let value = get_quoted_str(paris.next().unwrap())?;

    Ok(Directive::Option(Opt { name, value }))
}

fn custom_directive<'i>(directive: Pair<'i, Rule>, state: &ParseState) -> ParseResult<Directive> {
    let source = directive.as_str();
    let mut pairs = directive.into_inner();
    Ok(Directive::Custom(Custom {
        meta: Metadata::new(),
        date: date(pairs.next().unwrap())?,
        name: get_quoted_str(pairs.next().unwrap())?,
        values: {
            match pairs.peek() {
                None => {
                    Vec::new()
                }
                Some(ref p) => {
                    if pairs.peek().unwrap().as_rule() == Rule::custom_value_list {
                        pairs.peek().unwrap().into_inner()
                            .map(|p| may_quoted_str(p))
                            .collect::<ParseResult<Vec<_>>>()?
                    } else {
                        Vec::new()
                    }
                }
            }
        },
    }))
}

fn may_quoted_str<'i>(pair: Pair<'i, Rule>) -> ParseResult<String> {
    debug_assert!(pair.as_rule() == Rule::quoted_str || pair.as_rule() == Rule::unquoted_str || );
    if pair.as_rule() == Rule::quoted_str {
        return get_quoted_str(pair);
    }

    return Ok(pair.as_str().into());
}

fn get_quoted_str<'i>(pair: Pair<'i, Rule>) -> ParseResult<String> {
    debug_assert!(pair.as_rule() == Rule::quoted_str);
    let span = pair.as_span();
    Ok(pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::invalid_state_with_span("quoted string", span))?
        .as_str()
        .into())
}

fn plugin_directive<'i>(directive: Pair<'i, Rule>) -> ParseResult<Directive> {
    let mut paris = directive.into_inner();

    let name = get_quoted_str(paris.next().unwrap())?;
    let value = paris.next().map(get_quoted_str).transpose()?;

    Ok(Directive::Plugin(Plugin {
        module: name,
        config: value,
    }))
}

fn date<'i>(pair: Pair<'i, Rule>) -> ParseResult<NaiveDate> {
    let mut pairs = pair.into_inner();

    let year = pairs.next().unwrap().as_str().parse().unwrap();
    pairs.next();
    let mon = pairs.next().unwrap().as_str().parse().unwrap();
    pairs.next();
    let day = pairs.next().unwrap().as_str().parse().unwrap();

    Ok(NaiveDate::from_ymd_opt(year, mon, day).unwrap())

    // NaiveDate::from_ymd_opt(pair.as_str()).ok_or_else(|| ParseError {
    //     kind: ParseErrorKind::InvalidParserState {
    //         message: "invalid date".to_string(),
    //     },
    //     location: pair.as_span().start_pos().line_col(),
    //     source: pair.as_str().into(),
    // })
}

fn open_directive<'i>(directive: Pair<'i, Rule>, state: &ParseState) -> ParseResult<Directive> {
    let span = directive.as_span();

    let mut pairs = directive.into_inner();

    Ok(Directive::Open(Open {
        meta: Metadata::new(),
        date: date(pairs.next().unwrap())?,
        account: account(pairs.next().unwrap())?,
        currencies: match pairs.peek() {
            Some(ref p) => {
                if p.as_rule() == Rule::commodity_list {
                    pairs
                        .next()
                        .ok_or_else(|| {
                            ParseError::invalid_state_with_span(
                                stringify!(currencies),
                                span.clone(),
                            )
                        })?
                        .into_inner()
                        .map(|x| x.as_str().to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            None => Vec::new(),
        },
        booking: match pairs.peek() {
            Some(ref p) => {
                if p.as_rule() == Rule::quoted_str {
                    let f = {
                        |p: Pair<'i, _>| -> ParseResult<Option<data::Booking>> {
                            let span = p.as_span();
                            get_quoted_str(p)?
                                .try_into()
                                .map_err(|_| {
                                    ParseError::invalid_input_with_span(
                                        format!("unknown booking method {}", span.as_str()),
                                        span,
                                    )
                                })
                                .map(Some)
                        }
                    };
                    let pair = pairs.next().ok_or_else(|| {
                        ParseError::invalid_state_with_span(stringify!(booking), span.clone())
                    })?;
                    f(pair)?
                } else {
                    None
                }
            }
            None => None,
        },
    }))
}

fn account<'i>(pair: Pair<'i, Rule>) -> ParseResult<String> {
    // debug_assert!(pair.as_rule() == Rule::account);
    // let span = pair.as_span();
    // let mut inner = pair.into_inner();
    // let first_pair = inner
    //     .next()
    //     .ok_or_else(|| ParseError::invalid_state_with_span("first part of account name", span))?;
    let first = pair.as_str();
    return Ok(first.into());
    //     return Ok(state
    //         .root_names
    //         .iter()
    //         .filter(|(_, ref v)| *v == first)
    //         .map(|(k, _)| k.clone())
    //         .next()
    //         .ok_or_else(|| {
    //             pest::error::Error::new_from_span(
    //                 pest::error::ErrorVariant::CustomError {
    //                     message: "Invalid root account".to_string(),
    //                 },
    //                 first_pair.as_span(),
    //             )
    //         })?);
}
