use crate::data::Amount;
use crate::{data, ParserError};
use beancount_parser::{BeancountFile, DirectiveContent};
use chrono::NaiveDate;
use pyo3::prelude::*;
// #[pyclass(subclass, module = "beancount.parser._parser")]
// #[derive(Clone, Debug)]
// struct Parser {
//     name: String,
// }
//
// #[allow(missing_docs)]
// #[derive(Debug, Clone, PartialEq)]
// #[non_exhaustive]
// pub enum DirectiveContent<Decimal> {
//     Transaction(Transaction),
//     Price(Price<Decimal>),
//     Balance(Balance<Decimal>),
//     Open(Open),
//     Close(Close),
//     Pad(Pad),
//     Commodity(Currency),
//     Event(Event),
// }
//
// #[pyclass(module = "beancount.parser._parser")]
// #[derive(Debug, Clone, PartialEq)]
// pub struct Transaction {
//     /// Transaction flag (`*` or `!` or `None` when using the `txn` keyword)
//     pub flag: Option<char>,
//     /// Payee (if present)
//     pub payee: Option<String>,
//     /// Narration (if present)
//     pub narration: Option<String>,
//     /// Set of tags
//     pub tags: HashSet<Tag>,
//     /// Set of links
//     pub links: HashSet<Link>,
//     /// Postings
//     pub postings: Vec<Posting<decimal::Decimal>>,
// }
//
// #[pymethods]
// impl Transaction {
//     #[new]
//     fn new(
//         flag: Option<char>,
//         payee: Option<String>,
//         narration: Option<String>,
//         tags: Option<HashSet<Tag>>,
//         links: Option<HashSet<Link>>,
//         postings: Option<Vec<Posting<decimal::Decimal>>>,
//     ) -> PyResult<Self> {
//         return Ok(Transaction {
//             flag,
//             payee,
//             narration,
//             tags: tags.or_else(HashSet::<Tag>::new()),
//             links: links.or_else(HashSet::<Link>::new()),
//             postings: postings.or_else(Vec::<Posting<decimal::Decimal>>::new()),
//         });
//     }
// }
//
// #[pymethods]
// impl Parser {
//     #[new]
//     fn new() -> PyResult<Self> {
//         return Ok(Parser {
//             name: String::from("test"),
//         });
//     }
//
//     // fn parse(&self, content: &str) -> PyResult<Vec<Directive<Decimal>>> {
//     //     let result = content.parse::<BeancountFile<Decimal>>();
//     //     match result {
//     //         Ok(file) => {
//     //             Ok(file.directives)
//     //         }
//     //         Err(err) => {
//     //             Err(ParserError::new_err(err.to_string()))
//     //         }
//     //     }
//     // }
// }

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
    S(String),
}

impl IntoPy<Py<PyAny>> for Directive {
    fn into_py(self, py: Python) -> Py<PyAny> {
        return match self {
            Directive::Open(x) => x.into_py(py),
            Directive::Close(x) => x.into_py(py),
            Directive::Commodity(x) => x.into_py(py),
            Directive::Pad(x) => x.into_py(py),
            Directive::Balance(x) => x.into_py(py),
            Directive::S(x) => x.into_py(py),
            Directive::Transaction(x) => x.into_py(py),
            Directive::Price(x) => x.into_py(py),
            Directive::Event(x) => x.into_py(py),
        };
    }
}

// #[pymethods]
// impl Directive {
//     fn __repr__(&self) -> String {
//         return format!(
//             "Directive(date={}, content={:?}, metadata={:#?}, line_number={:?})",
//             self.date,
//             self.content,
//             self.metadata,
//             self.line_number
//         );
//     }
// }
fn convert(x: beancount_parser::Directive<rust_decimal::Decimal>) -> Result<Directive, PyErr> {
    let date = NaiveDate::from_ymd_opt(x.date.year as i32, x.date.month as u32, x.date.day as u32);
    return match date {
        None => Err(ParserError::new_err(format!(
            "Invalid date {:#?} at lino {}",
            x.date, x.line_number
        ))),
        Some(date) => {
            match x.content {
                DirectiveContent::Open(ref v) => {
                    Ok(Directive::Open(data::Open {
                        meta: x
                            .metadata
                            .iter()
                            .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                            .collect(),
                        date,
                        account: v.account.to_string(),
                        currencies: v.currencies.iter().map(|x| x.to_string()).collect(),
                        booking: None,
                        // booking: v.booking_method,
                    }))
                }

                DirectiveContent::Close(ref v) => Ok(Directive::Close(data::Close {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    account: v.account.to_string(),
                })),

                DirectiveContent::Commodity(ref v) => Ok(Directive::Commodity(data::Commodity {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    currency: v.to_string(),
                })),

                DirectiveContent::Pad(ref v) => Ok(Directive::Pad(data::Pad {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    account: v.account.to_string(),
                    source_account: v.source_account.to_string(),
                })),

                DirectiveContent::Price(ref v) => Ok(Directive::Price(data::Price {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    currency: v.currency.to_string(),
                    amount: Amount { number: v.amount.value, currency: v.amount.currency.to_string() },
                })),

                DirectiveContent::Balance(ref v) => Ok(Directive::Balance(data::Balance {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    tolerance: v.tolerance.map(|x| x.into()),
                    diff_amount: None,
                    account: v.account.to_string(),
                    amount: Amount::from(&v.amount),
                })),
                DirectiveContent::Event(ref v) => Ok(Directive::Event(data::Event {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    typ: v.name.clone(),
                    description: v.value.clone(),
                })),
                DirectiveContent::Transaction(ref v) => Ok(Directive::Transaction(data::Transaction {
                    meta: x
                        .metadata
                        .iter()
                        .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                        .collect(),
                    date,
                    flag: v.flag.unwrap_or('*'),
                    payee: v.payee.clone(),
                    narration: v.narration.clone().unwrap_or("".to_string()),
                    tags: v.tags.iter().map(|x| x.to_string()).collect(),
                    links: v.links.iter().map(|x| x.to_string()).collect(),
                    postings: v
                        .postings
                        .iter()
                        .map(|x| data::Posting {
                            metadata: x
                                .metadata
                                .iter()
                                .map(|entry| (entry.0.to_string(), format!("{:?}", entry.1)))
                                .collect(),
                            account: x.account.to_string(),
                            units: None,
                            cost: None,
                            price: None,
                            flag: x.flag,
                        })
                        .collect(),
                })),

                _ => Ok(Directive::S("Unspported".to_string())),
            }

            // Ok(Directive::Open(data::Open {
            //     meta: Metadata::new(),
            //     date,
            //     account: format!("{:#?}", x),
            //     booking: Some(Booking::FIFO),
            //     currencies: vec!["CNY".to_string()],
            // account: x.account.to_string(),
            // currencies: x.currencies.iter().map(|x| x.to_string()).collect(),
            // booking: x.booking.map(|x| x.to_string()),
            // }))
            // Directive {
            //     date,
            //     line_number: x.line_number,
            // content: convert(x),
            // }
        }
    };
    // return match x.content {
    //     beancount_parser::DirectiveContent::Open(ref v) => {
    //         Directive::Open(data::Open {
    //             account: v.account.to_string(),
    //             currencies: v.currencies.iter().map(|x| x.to_string()).collect(),
    // booking: v.booking.map(|x| x.to_string()),
    // meta: x.metadata,
    // })
    // }
    // beancount_parser::DirectiveContent::Close(ref v) => {
    //     Directive::Close(data::Close {
    //         account: v.account.to_string(),
    //         metadata: x.metadata,
    //     })
    // }
    // _ => panic!("Unsupported directive"),
    // };
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
