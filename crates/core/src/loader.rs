use crate::Directive;
use crate::path_utils::resolve_path;
use beancount_parser::ParseError;
use chrono::NaiveDate;
use glob::glob;
use path_clean::PathClean;
use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::booking::{BookingConfig, BookingError, book_directives};

fn read_file_to_string_auto(filename: &str) -> std::io::Result<String> {
  if crate::encryption::is_encrypted_file(filename) {
    crate::encryption::read_encrypted_file(filename)
  } else {
    std::fs::read_to_string(filename)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderError {
  pub filename: String,
  pub line: u32,
  pub column: u32,
  pub message: String,
}

impl LoaderError {
  fn at_load(message: String) -> Self {
    Self {
      filename: "<load>".to_string(),
      line: 0,
      column: 0,
      message,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUnit {
  pub filename: String,
  pub includes: Vec<String>,
  pub directives: Vec<Directive>,
  pub options: Vec<crate::OptionDirective>,
  pub plugins: Vec<crate::Plugin>,
  pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadResult {
  pub units: Vec<ParsedUnit>,
  pub directives: Vec<Directive>,
  pub parse_errors: Vec<ParseError>,
  pub load_errors: Vec<LoaderError>,
  pub filenames_seen: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadAndBookResult {
  pub load: LoadResult,
  pub booked_directives: Vec<Directive>,
  pub booking_errors: Vec<BookingError>,
}

fn directive_date(d: &Directive) -> &str {
  match d {
    Directive::Open(x) => &x.date,
    Directive::Close(x) => &x.date,
    Directive::Balance(x) => &x.date,
    Directive::Pad(x) => &x.date,
    Directive::Transaction(x) => &x.date,
    Directive::Commodity(x) => &x.date,
    Directive::Price(x) => &x.date,
    Directive::Event(x) => &x.date,
    Directive::Query(x) => &x.date,
    Directive::Note(x) => &x.date,
    Directive::Document(x) => &x.date,
    Directive::Custom(x) => &x.date,
    // Non-entry directives shouldn't reach the sorted list, but keep a stable fallback.
    _ => "1970-01-01",
  }
}

fn directive_lineno(d: &Directive) -> u32 {
  match d {
    Directive::Open(x) => x.meta.line as u32,
    Directive::Close(x) => x.meta.line as u32,
    Directive::Balance(x) => x.meta.line as u32,
    Directive::Pad(x) => x.meta.line as u32,
    Directive::Transaction(x) => x.meta.line as u32,
    Directive::Commodity(x) => x.meta.line as u32,
    Directive::Price(x) => x.meta.line as u32,
    Directive::Event(x) => x.meta.line as u32,
    Directive::Query(x) => x.meta.line as u32,
    Directive::Note(x) => x.meta.line as u32,
    Directive::Document(x) => x.meta.line as u32,
    Directive::Custom(x) => x.meta.line as u32,
    _ => 0,
  }
}

fn directive_sort_order(d: &Directive) -> i32 {
  match d {
    Directive::Open(_) => -2,
    Directive::Balance(_) => -1,
    Directive::Document(_) => 1,
    Directive::Close(_) => 2,
    _ => 0,
  }
}

fn directive_sort_key(d: &Directive) -> (NaiveDate, i32, u32) {
  let date = NaiveDate::parse_from_str(directive_date(d), "%Y-%m-%d")
    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
  (date, directive_sort_order(d), directive_lineno(d))
}

fn partition_directives(
  directives: Vec<Directive>,
) -> (
  Vec<String>,
  Vec<Directive>,
  Vec<crate::OptionDirective>,
  Vec<crate::Plugin>,
) {
  let mut includes = Vec::new();
  let mut filtered = Vec::new();
  let mut options = Vec::new();
  let mut plugins = Vec::new();

  for directive in directives {
    match directive {
      Directive::Include(include) => includes.push(include.filename.clone()),
      Directive::Option(opt) => options.push(opt),
      Directive::Plugin(plugin) => plugins.push(plugin),
      Directive::PushTag(_)
      | Directive::PopTag(_)
      | Directive::PushMeta(_)
      | Directive::PopMeta(_)
      | Directive::Headline(_)
      | Directive::Comment(_)
      | Directive::Raw(_) => filtered.push(directive),
      other => filtered.push(other),
    }
  }
  (includes, filtered, options, plugins)
}

fn apply_meta_to_key_values(
  mut key_values: crate::SmallKeyValues,
  active_meta: &BTreeMap<String, Vec<crate::KeyValue>>,
) -> crate::SmallKeyValues {
  use std::collections::BTreeSet;

  let present: BTreeSet<String> = key_values.iter().map(|kv| kv.key.clone()).collect();

  for (key, stack) in active_meta {
    if present.contains(key) {
      continue;
    }
    if let Some(kv) = stack.last() {
      key_values.push(kv.clone());
    }
  }

  key_values
}

fn apply_meta_to_directive(
  directive: Directive,
  active_meta: &BTreeMap<String, Vec<crate::KeyValue>>,
) -> Directive {
  match directive {
    Directive::Open(mut open) => {
      open.key_values = apply_meta_to_key_values(open.key_values, active_meta);
      Directive::Open(open)
    }
    Directive::Close(mut close) => {
      close.key_values = apply_meta_to_key_values(close.key_values, active_meta);
      Directive::Close(close)
    }
    Directive::Balance(mut bal) => {
      bal.key_values = apply_meta_to_key_values(bal.key_values, active_meta);
      Directive::Balance(bal)
    }
    Directive::Pad(mut pad) => {
      pad.key_values = apply_meta_to_key_values(pad.key_values, active_meta);
      Directive::Pad(pad)
    }
    Directive::Commodity(mut comm) => {
      comm.key_values = apply_meta_to_key_values(comm.key_values, active_meta);
      Directive::Commodity(comm)
    }
    Directive::Price(mut price) => {
      price.key_values = apply_meta_to_key_values(price.key_values, active_meta);
      Directive::Price(price)
    }
    Directive::Event(mut event) => {
      event.key_values = apply_meta_to_key_values(event.key_values, active_meta);
      Directive::Event(event)
    }
    Directive::Query(mut query) => {
      query.key_values = apply_meta_to_key_values(query.key_values, active_meta);
      Directive::Query(query)
    }
    Directive::Note(mut note) => {
      note.key_values = apply_meta_to_key_values(note.key_values, active_meta);
      Directive::Note(note)
    }
    Directive::Document(mut doc) => {
      doc.key_values = apply_meta_to_key_values(doc.key_values, active_meta);
      Directive::Document(doc)
    }
    Directive::Custom(mut custom) => {
      custom.key_values = apply_meta_to_key_values(custom.key_values, active_meta);
      Directive::Custom(custom)
    }
    other => other,
  }
}

fn apply_push_tags_and_meta(
  directives: Vec<Directive>,
) -> (Vec<Directive>, Vec<ParseError>) {
  use std::collections::BTreeSet;

  let mut out: Vec<Directive> = Vec::new();
  let mut errors: Vec<ParseError> = Vec::new();
  let mut active_tags: BTreeSet<String> = BTreeSet::new();
  let mut active_meta: BTreeMap<String, Vec<crate::KeyValue>> = BTreeMap::new();

  for directive in directives {
    match directive {
      Directive::PushMeta(pm) => {
        let kv = crate::KeyValue {
          span: pm.span,
          key: pm.key.clone(),
          value: pm.value.clone(),
        };
        active_meta.entry(pm.key).or_default().push(kv);
      }
      Directive::PopMeta(pm) => match active_meta.get_mut(&pm.key) {
        Some(stack) => {
          if stack.pop().is_none() {
            errors.push(ParseError {
              line: pm.meta.line,
              column: pm.meta.column,
              message: format!("Attempting to pop absent metadata key: '{}'", pm.key),
            });
          }
          if stack.is_empty() {
            active_meta.remove(&pm.key);
          }
        }
        None => {
          errors.push(ParseError {
            line: pm.meta.line,
            column: pm.meta.column,
            message: format!("Attempting to pop absent metadata key: '{}'", pm.key),
          });
        }
      },
      Directive::PushTag(tag) => {
        active_tags.insert(tag.tag.clone());
      }
      Directive::PopTag(tag) => {
        if !active_tags.remove(&tag.tag) {
          errors.push(ParseError {
            line: tag.meta.line,
            column: tag.meta.column,
            message: format!("Attempting to pop absent tag: '{}'", tag.tag),
          });
        }
      }
      Directive::Transaction(mut txn) => {
        txn.key_values = apply_meta_to_key_values(txn.key_values, &active_meta);
        if !active_tags.is_empty() {
          let mut tag_set: BTreeSet<String> = txn.tags.iter().cloned().collect();
          tag_set.extend(active_tags.iter().cloned());
          txn.tags = tag_set.into_iter().collect();
        }
        out.push(Directive::Transaction(txn));
      }
      Directive::Document(mut doc) => {
        doc.key_values = apply_meta_to_key_values(doc.key_values, &active_meta);
        if !active_tags.is_empty() {
          let mut tag_set: BTreeSet<String> = doc.tags.iter().cloned().collect();
          tag_set.extend(active_tags.iter().cloned());
          doc.tags = tag_set.into_iter().collect();
        }
        out.push(Directive::Document(doc));
      }
      Directive::Comment(_) | Directive::Headline(_) => {
        // Ignore in Rust loader.
      }
      Directive::Raw(raw) => {
        errors.push(ParseError {
          line: raw.meta.line,
          column: raw.meta.column,
          message: format!("Unrecognized directive: {}", raw.text),
        });
      }
      other => {
        out.push(apply_meta_to_directive(other, &active_meta));
      }
    }
  }

  if !active_tags.is_empty() {
    for tag in active_tags {
      errors.push(ParseError {
        line: 0,
        column: 0,
        message: format!("Unbalanced pushed tag: '{}'", tag),
      });
    }
  }

  if !active_meta.is_empty() {
    for key in active_meta.keys() {
      errors.push(ParseError {
        line: 0,
        column: 0,
        message: format!("Unbalanced metadata key: '{}'", key),
      });
    }
  }

  (out, errors)
}

fn parse_content_to_unit(filename: &str, content: &str) -> ParsedUnit {
  let directives = beancount_parser::parse_lossy(content);

  let normalized = match crate::normalize_directives(&directives, filename, content) {
    Ok(normalized) => normalized,
    Err(err) => {
      return ParsedUnit {
        filename: filename.to_string(),
        includes: Vec::new(),
        directives: Vec::new(),
        options: Vec::new(),
        plugins: Vec::new(),
        errors: vec![err],
      };
    }
  };

  let (includes, filtered, options, plugins) = partition_directives(normalized);
  let (cleaned, mut errors) = apply_push_tags_and_meta(filtered);

  ParsedUnit {
    filename: filename.to_string(),
    includes,
    directives: cleaned,
    options,
    plugins,
    errors: {
      let mut all = Vec::new();
      all.append(&mut errors);
      all
    },
  }
}

/// Recursively load a file, following `include` directives (with glob expansion).
///
/// Returns raw Rust directives (already normalized and with push-tag/meta applied
/// per file), plus parse/load errors and metadata for options/plugins per unit.
pub fn load_file_recursive(top_filename: &str) -> std::io::Result<LoadResult> {
  load_sources_recursive(vec![(top_filename.to_string(), true)], None)
}

/// Recursively load a string source; includes resolve relative to CWD.
pub fn load_string_recursive(content: &str, filename: &str) -> LoadResult {
  load_sources_recursive(vec![(filename.to_string(), false)], Some(content))
    .expect("string load should not perform IO")
}

/// Convenience API for Rust callers: recursive load + booking.
///
/// Booking is performed on the fully aggregated directive stream (after include expansion
/// and sorting). The current booking engine is still under migration.
pub fn load_file_and_book(
  top_filename: &str,
  config: &BookingConfig,
) -> std::io::Result<LoadAndBookResult> {
  let load = load_file_recursive(top_filename)?;
  Ok(book_loaded(load, config))
}

/// Convenience API for Rust callers: recursive load (string source) + booking.
pub fn load_string_and_book(
  content: &str,
  filename: &str,
  config: &BookingConfig,
) -> LoadAndBookResult {
  let load = load_string_recursive(content, filename);
  book_loaded(load, config)
}

/// Book directives from an already-loaded `LoadResult`.
pub fn book_loaded(load: LoadResult, config: &BookingConfig) -> LoadAndBookResult {
  let (booked_directives, booking_errors) =
    book_directives(load.directives.clone(), config);
  LoadAndBookResult {
    load,
    booked_directives,
    booking_errors,
  }
}

fn load_sources_recursive(
  sources: Vec<(String, bool)>,
  string_source: Option<&str>,
) -> std::io::Result<LoadResult> {
  let mut units: Vec<ParsedUnit> = Vec::new();
  let mut directives: Vec<Directive> = Vec::new();
  let mut parse_errors: Vec<ParseError> = Vec::new();
  let mut load_errors: Vec<LoaderError> = Vec::new();

  let mut source_stack: VecDeque<(String, bool)> = VecDeque::from(sources);
  let mut filenames_seen: HashSet<String> = HashSet::new();

  while let Some((source, is_file)) = source_stack.pop_front() {
    if is_file {
      let filename = std::path::Path::new(&source)
        .to_path_buf()
        .clean()
        .to_string_lossy()
        .into_owned();

      if filenames_seen.contains(&filename) {
        load_errors.push(LoaderError::at_load(format!(
          "Duplicate filename parsed: \"{}\"",
          filename
        )));
        continue;
      }

      if !std::path::Path::new(&filename).exists() {
        load_errors.push(LoaderError::at_load(format!(
          "File \"{}\" does not exist",
          filename
        )));
        continue;
      }

      filenames_seen.insert(filename.clone());

      let content = read_file_to_string_auto(&filename)?;
      let unit = parse_content_to_unit(&filename, &content);

      // Expand includes from this unit.
      let mut include_expanded: Vec<String> = Vec::new();
      for include in &unit.includes {
        let search_path = resolve_path(&filename, include);
        let mut matched: Vec<String> = Vec::new();
        for entry in glob(&search_path)
          .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.msg))?
        {
          if let Ok(path) = entry {
            matched.push(path.to_string_lossy().into_owned());
          }
        }

        if matched.is_empty() {
          load_errors.push(LoaderError::at_load(format!(
            "File glob \"{}\" does not match any files",
            include
          )));
        } else {
          include_expanded.extend(matched);
        }
      }

      for inc in include_expanded {
        let norm = std::path::Path::new(&inc)
          .to_path_buf()
          .clean()
          .to_string_lossy()
          .into_owned();
        source_stack.push_back((norm, true));
      }

      parse_errors.extend(unit.errors.clone());
      directives.extend(unit.directives.clone());
      units.push(unit);
    } else {
      // Only supported for the initial top-level string.
      let content = string_source.unwrap_or("");
      let unit = parse_content_to_unit(&source, content);

      // Includes resolve with resolve_path's "<string>" behavior.
      let mut include_expanded: Vec<String> = Vec::new();
      for include in &unit.includes {
        let search_path = resolve_path(&source, include);
        let mut matched: Vec<String> = Vec::new();
        for entry in glob(&search_path)
          .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.msg))?
        {
          if let Ok(path) = entry {
            matched.push(path.to_string_lossy().into_owned());
          }
        }

        if matched.is_empty() {
          load_errors.push(LoaderError::at_load(format!(
            "File glob \"{}\" does not match any files",
            include
          )));
        } else {
          include_expanded.extend(matched);
        }
      }

      for inc in include_expanded {
        let norm = std::path::Path::new(&inc)
          .to_path_buf()
          .clean()
          .to_string_lossy()
          .into_owned();
        source_stack.push_back((norm, true));
      }

      parse_errors.extend(unit.errors.clone());
      directives.extend(unit.directives.clone());
      units.push(unit);
    }
  }

  let mut filenames_seen_vec: Vec<String> = filenames_seen.into_iter().collect();
  filenames_seen_vec.sort();

  directives.sort_by(|a, b| directive_sort_key(a).cmp(&directive_sort_key(b)));

  Ok(LoadResult {
    units,
    directives,
    parse_errors,
    load_errors,
    filenames_seen: filenames_seen_vec,
  })
}
