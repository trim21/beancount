use pyo3::create_exception;
use pyo3::prelude::*;
use std::prelude::*;

mod data;
mod error;
mod decimal;
mod parse;
mod parser;

#[pymodule]
fn __beancount(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ParserError", m.py().get_type_bound::<ParserError>())?;

    m.add_class::<data::Booking>()?;
    m.add_class::<data::Open>()?;
    m.add_class::<data::Close>()?;
    m.add_class::<data::Pad>()?;
    m.add_class::<data::Price>()?;
    m.add_class::<data::Amount>()?;

    m.add_class::<data::Cost>()?;
    m.add_class::<data::CostSpec>()?;
    m.add_class::<data::Posting>()?;
    m.add_class::<data::Transaction>()?;
    m.add_function(wrap_pyfunction!(parse::parse, m)?)?;
    m.add_function(wrap_pyfunction!(parser::py_parse2, m)?)?;

    // parser::register_child_module(m)
    return Ok(());
}

create_exception!(
    beancount.parser._parser,
    ParserError,
    pyo3::exceptions::PyException
);
