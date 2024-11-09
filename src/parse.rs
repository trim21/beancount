use crate::data::Amount;
use crate::decimal::Decimal;
use pyo3::prelude::*;

use crate::{data, ParserError};
use beancount_parser::metadata::Value;
use beancount_parser::{BeancountFile, DirectiveContent};
use chrono::NaiveDate;

#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct Opt {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub value: String,
}

#[pymethods]
impl Opt {
    fn __str__(&self) -> String {
        return format!("Option(name={:?}, value={:?}", self.name, self.value);
    }

    fn __repr__(&self) -> String {
        return self.__str__();
    }
}

#[pyclass]
pub struct File {
    #[pyo3(get)]
    // we force beancount format to be valid utf8
    pub includes: Vec<String>,

    #[pyo3(get)]
    pub options: Vec<Opt>,

    #[pyo3(get)]
    pub directives: Vec<Directive>,
}

// #[pyclass]
#[derive(Debug, Clone)]
pub enum Directive {
    Open(data::Open),
    Close(data::Close),
    Commodity(data::Commodity),
    Transaction(data::Transaction),
    Pad(data::Pad),
    Balance(data::Balance),
    Price(data::Price),
    Event(data::Event),
    Plugin(data::Plugin),
    Option(Opt),
    S(String),
    Custom(data::Custom),
}

impl IntoPy<Py<PyAny>> for Directive {
    fn into_py(self, py: Python) -> Py<PyAny> {
        match self {
            Directive::Open(x) => x.into_py(py),
            Directive::Close(x) => x.into_py(py),
            Directive::Commodity(x) => x.into_py(py),
            Directive::Custom(x) => x.into_py(py),
            Directive::Transaction(x) => x.into_py(py),
            Directive::Pad(x) => x.into_py(py),
            Directive::Balance(x) => x.into_py(py),
            Directive::Price(x) => x.into_py(py),
            Directive::Event(x) => x.into_py(py),
            Directive::Plugin(x) => x.into_py(py),
            Directive::Option(x) => x.into_py(py),
            Directive::S(x) => x.into_py(py),
        }
    }
}

fn convert_date(x: &beancount_parser::Date) -> Result<NaiveDate, PyErr> {
    match NaiveDate::from_ymd_opt(x.year as i32, x.month as u32, x.day as u32) {
        None => Err(ParserError::new_err(format!("Invalid date {:#?}", x))),
        Some(date) => Ok(date),
    }
}

fn convert_metadata(x: &beancount_parser::Directive<Decimal>) -> data::Metadata {
    let mut h: data::Metadata = x
        .metadata
        .iter()
        .map(|entry| {
            (
                entry.0.to_string(),
                match entry.1 {
                    Value::String(s) => s.to_string(),
                    Value::Number(s) => s.to_string(),
                    Value::Currency(s) => s.to_string(),
                    _ => format!("{:?}", entry.1).to_string(),
                },
            )
        })
        .collect();

    h.insert("lineno".to_string(), x.line_number.to_string());

    return h;
}

fn convert(x: beancount_parser::Directive<rust_decimal::Decimal>) -> Result<Directive, PyErr> {
    let date = convert_date(&x.date)?;
    return match x.content {
        DirectiveContent::Open(ref v) => {
            Ok(Directive::Open(data::Open {
                meta: convert_metadata(&x),
                date,
                account: v.account.to_string(),
                currencies: v.currencies.iter().map(|x| x.to_string()).collect(),
                booking: None,
                // booking: v.booking_method,
            }))
        }

        DirectiveContent::Close(ref v) => Ok(Directive::Close(data::Close {
            meta: convert_metadata(&x),
            date,
            account: v.account.to_string(),
        })),

        DirectiveContent::Commodity(ref v) => Ok(Directive::Commodity(data::Commodity {
            meta: convert_metadata(&x),
            date,
            currency: v.to_string(),
        })),

        DirectiveContent::Pad(ref v) => Ok(Directive::Pad(data::Pad {
            meta: convert_metadata(&x),
            date,
            account: v.account.to_string(),
            source_account: v.source_account.to_string(),
        })),

        DirectiveContent::Price(ref v) => Ok(Directive::Price(data::Price {
            meta: convert_metadata(&x),
            date,
            currency: v.currency.to_string(),
            amount: Amount {
                number: v.amount.value,
                currency: v.amount.currency.to_string(),
            },
        })),

        DirectiveContent::Balance(ref v) => Ok(Directive::Balance(data::Balance {
            meta: convert_metadata(&x),
            date,
            tolerance: v.tolerance.map(|x| x.into()),
            diff_amount: None,
            account: v.account.to_string(),
            amount: Amount::from(&v.amount),
        })),
        DirectiveContent::Event(ref v) => Ok(Directive::Event(data::Event {
            meta: convert_metadata(&x),
            date,
            typ: v.name.clone(),
            description: v.value.clone(),
        })),
        DirectiveContent::Transaction(ref v) => Ok(Directive::Transaction(data::Transaction {
            meta: convert_metadata(&x),
            date,
            flag: v.flag.unwrap_or('*'),
            payee: v.payee.clone(),
            narration: v.narration.clone().unwrap_or("".to_string()),
            tags: v.tags.iter().map(|x| x.to_string()).collect(),
            links: v.links.iter().map(|x| x.to_string()).collect(),
            postings: v
                .postings
                .iter()
                .map(|x| x.try_into())
                .collect::<Result<Vec<_>, PyErr>>()?,
        })),

        _ => Ok(Directive::S("Unspported".to_string())),
    };
}

#[pyfunction]
pub fn parse(content: &str) -> PyResult<File> {
    let result = content.parse::<BeancountFile<rust_decimal::Decimal>>();
    return match result {
        Ok(bean) => {
            let mut directives = Vec::with_capacity(bean.directives.len());

            let mut errors = Vec::new();

            for x in bean.directives {
                match convert(x) {
                    Ok(x) => directives.push(x),
                    Err(err) => errors.push(err),
                }
            }

            Ok(File {
                includes: bean
                    .includes
                    .iter()
                    .map(|x| x.to_str().unwrap().to_string())
                    .collect(),
                options: bean
                    .options
                    .iter()
                    .map(|x| Opt {
                        name: x.name.to_string(),
                        value: x.value.to_string(),
                    })
                    .collect(),
                directives,
            })
        }
        Err(err) => Err(ParserError::new_err(err.to_string())),
    };
}
