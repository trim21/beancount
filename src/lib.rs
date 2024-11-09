#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use pyo3::create_exception;
use pyo3::prelude::*;

mod data;
mod decimal;
mod error;
mod parse;
mod parse2;

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
    m.add_class::<data::Plugin>()?;
    m.add_function(wrap_pyfunction!(parse::parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse2::py_parse2, m)?)?;

    // parser::register_child_module(m)
    return Ok(());
}

create_exception!(
    beancount.parser._parser,
    ParserError,
    pyo3::exceptions::PyException
);
