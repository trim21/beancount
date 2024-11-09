use crate::error::{ParseError, ParseResult};
use crate::parse::{Directive, File, Opt};
use crate::ParserError;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use pyo3::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "./grammar.pest"] // relative to src
pub struct MyParser;

#[derive(Debug, Default)]
struct ParseState<'i> {
    root_names: HashMap<String, String>,

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

    let mut state = ParseState::default();

    let mut directives = Vec::new();

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
            _ => {
                let dir = directive(entry, &state)?;

                directives.push(dir);
            }
        }
    }

    return Ok(File {
        includes: vec![],
        options: vec![],
        directives,
    });
}

fn directive(directive: Pair<Rule>, state: &ParseState) -> ParseResult<Directive> {
    let dir = match directive.as_rule() {
        Rule::option => option_directive(directive)?,
        // Rule::plugin => plugin_directive(directive)?,
        // Rule::custom => custom_directive(directive, state)?,
        // Rule::include => include_directive(directive)?,
        // Rule::open => open_directive(directive, state)?,
        // Rule::close => close_directive(directive, state)?,
        // Rule::commodity_directive => commodity_directive(directive, state)?,
        // Rule::note => note_directive(directive, state)?,
        // Rule::pad => pad_directive(directive, state)?,
        // Rule::query => query_directive(directive, state)?,
        // Rule::event => event_directive(directive, state)?,
        // Rule::document => document_directive(directive, state)?,
        // Rule::price => price_directive(directive, state)?,
        // Rule::transaction => transaction_directive(directive, state)?,
        _ => Directive::S(format!("UnSupported {:#?}", directive).into()),
    };
    Ok(dir)
}

fn option_directive<'i>(directive: Pair<'i, Rule>) -> ParseResult<Directive> {
    let mut paris = directive.into_inner();

    let name = get_quoted_str(paris.next().unwrap())?;
    let value = get_quoted_str(paris.next().unwrap())?;

    Ok(Directive::Option(Opt { name, value }))
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
