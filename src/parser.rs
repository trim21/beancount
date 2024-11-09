use crate::parse::File;
use crate::parse::Rule::entry;
use crate::ParserError;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use pyo3::prelude::*;
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
#[pyfunction]
pub fn parse2(content: &str) -> PyResult<File> {
    let entries = MyParser::parse(Rule::file, content).or_else(|e| {
        Err(ParserError::new_err(format!("Error parsing file: {}", e)))
    })?.next().ok_or_else(|| ParserError::new_err("non-empty parse result"))?;

    // println!("{:#?}", entries);

    let mut state = ParseState::default();

    let mut directives = Vec::new();


    for entry in entries.into_inner() {
        match entry.as_rule() {
            Rule::EOI => {
                break;
            }
            // Rule::pushta => {
            //     state.push_tag(extract_tag(directive_pair)?);
            // }
            // Rule::poptag => {
            //     let span = directive_pair.as_span();
            //     if let Err(msg) = state.pop_tag(extract_tag(directive_pair)?) {
            //         return Err(ParseError::invalid_input_with_span(msg, span));
            //     }
            // }
            _ => {
                let dir = directive(entry, &state)?;

                // Change the root account names on such an option:
                // option "name_assets" "Assets"
                if let bc::Directive::Option(ref opt) = dir {
                    if let Some((account_type, account_name)) = opt.root_name_change() {
                        state.root_names.insert(account_type, account_name);
                    }
                }

                directives.push(dir);
            }
        }
    }

    return Ok(File {
        includes: vec![],
        options: vec![],
        directives: vec![],
    });
}

fn directive<'i>(directive: Pair<'i, Rule>, state: &ParseState) -> ParseResult<bc::Directive<'i>> {
    let dir = match directive.as_rule() {
        // Rule::option => option_directive(directive)?,
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
        // _ => bc::Directive::Unsupported,
    };
    Ok(dir)
}
