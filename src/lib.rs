use pyo3::create_exception;
use pyo3::prelude::*;

mod data;
mod decimal;
mod parse;

#[pymodule]
fn __beancount(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ParserError", m.py().get_type_bound::<ParserError>())?;

    m.add_class::<data::Booking>()?;

    m.add_class::<data::Open>()?;
    m.add_class::<data::Price>()?;
    m.add_class::<data::Amount>()?;
    m.add_class::<data::PostingPrice>()?;

    m.add_class::<data::Cost>()?;
    m.add_class::<data::PostingPrice>()?;
    m.add_class::<data::Posting>()?;
    m.add_function(wrap_pyfunction!(parse::parse, m)?)?;

    // parser::register_child_module(m)
    return Ok(());
}

create_exception!(
    beancount.parser._parser,
    ParserError,
    pyo3::exceptions::PyException
);
