use crate::decimal::Decimal;
use crate::ParserError;
use chrono::NaiveDate;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyString};
use pyo3::{pyclass, pymethods, Bound, PyAny, PyResult};
use std::collections::{HashMap, HashSet};

pub type Metadata = HashMap<String, String>;

pub type Currency = String;

#[pyclass]
#[derive(Debug, Clone)]
pub struct Open {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub account: String,
    pub currencies: Vec<Currency>,
    pub booking: Option<Booking>,
}

#[pymethods]
impl Open {
    fn __str__(&self) -> String {
        return format!(
            "Open(meta={:?}, date={:?}, account={:?}, currencies={:?}, booking={:?})",
            self.meta,
            self.date,
            self.account,
            self.currencies,
            self.booking.as_ref().map(|x| x.__str__())
        );
    }

    fn __repr__(&self) -> String {
        return self.__str__();
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Close {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub account: String,
}

#[pymethods]
impl Close {
    fn __str__(&self) -> String {
        return format!(
            "Close(meta={:?}, date={}, account={})",
            self.meta, self.date, self.account
        );
    }

    fn __repr__(&self) -> String {
        return self.__str__();
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Commodity {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub currency: Currency,
}

#[pymethods]
impl Commodity {
    fn __repr__(&self) -> String {
        return format!(
            "Commodity(meta={:?}, date={}, currency={})",
            self.meta, self.date, self.currency
        );
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Pad {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub account: String,
    pub source_account: String,
}

#[pymethods]
impl Pad {
    fn __repr__(&self) -> String {
        return format!(
            "Pad(meta={:?}, date={}, account={}, source_account={})",
            self.meta, self.date, self.account, self.source_account
        );
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Balance {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub account: String,
    pub amount: Amount,
    pub tolerance: Option<rust_decimal::Decimal>,
    pub diff_amount: Option<Amount>,
}

// #[derive(Debug, Clone)]
// #[non_exhaustive]
#[pyclass]
#[derive(Debug, Clone)]
pub struct Posting {
    pub metadata: Metadata,
    pub account: String,
    pub units: Option<Amount>,
    pub cost: Option<PostingCost>,
    pub price: Option<Amount>,
    pub flag: Option<char>,
}
fn convert_date(x: &beancount_parser::Date) -> Result<NaiveDate, PyErr> {
    match NaiveDate::from_ymd_opt(x.year as i32, x.month as u32, x.day as u32) {
        None => Err(ParserError::new_err(format!("Invalid date {:#?}", x))),
        Some(date) => Ok(date),
    }
}
impl TryInto<Posting> for beancount_parser::Posting<Decimal> {
    type Error = PyErr;


    fn try_into(self) -> Result<Posting, Self::Error> {
        return Posting::try_from(&self);
    }
}

impl TryFrom<&beancount_parser::Posting<Decimal>> for Posting {
    type Error = PyErr;

    fn try_from(value: &beancount_parser::Posting<Decimal>) -> Result<Self, Self::Error> {
        let cost = match &value.cost {
            None => None,
            Some(c) => {
                Some(PostingCost::Cost(Cost {
                    date: convert_date(&c.date.unwrap())?,
                    number: c.amount.clone().unwrap().value,
                    currency: c.amount.clone().unwrap().currency.to_string(),
                    label: None,
                }))
            }
        };

        let price = value.price.clone();

        return Ok(Posting {
            metadata: value
                .metadata
                .iter()
                .map(|entry| {
                    (entry.0.to_string(), format!("{:?}", entry.1))
                })
                .collect(),
            account: value.account.to_string(),
            units: None,
            cost,
            price: None,
            flag: value.flag,
        });
    }
}

#[derive(Debug, Clone)]
pub enum PostingCost {
    Cost(Cost),
    CostSpec(CostSpec),
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Cost {
    pub date: NaiveDate,
    pub number: Decimal,
    pub currency: Currency,
    pub label: Option<String>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct CostSpec {
    pub number_per: Option<Decimal>,
    pub number_total: Option<Decimal>,
    pub currency: Option<Currency>,
    pub date: Option<NaiveDate>,
    pub label: Option<String>,
    pub merge: Option<bool>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Transaction {
    pub meta: Metadata,
    pub date: NaiveDate,
    pub flag: char,
    pub payee: Option<String>,
    pub narration: String,
    pub tags: HashSet<String>,
    pub links: HashSet<String>,
    pub postings: Vec<Posting>,
}

#[pymethods]
impl Transaction {
    fn __repr__(&self) -> String {
        return format!(
            "Transaction(meta={:?}, date={:?}, flag={:?}, payee={:?}, narration={:?}, tags={:?}, links={:?}, postings={:?})",
            self.meta,
            self.date,
            self.flag,
            self.payee.clone().unwrap_or("None".to_string()),
            self.narration,
            self.tags,
            self.links,
            self.postings
        );
    }
}

// #[pymethods]
// impl Posting {
//     #[new]
//     #[pyo3(signature = (flag, account, amount=None, cost=None, price=None, metadata=None))]
//     fn new(
//         flag: Option<char>,
//         account: String,
//         amount: Option<Amount>,
//         cost: Option<Cost>,
//         price: Option<PostingPrice>,
//         metadata: Option<Metadata>,
//     ) -> PyResult<Self> {
//         return Ok(Posting {
//             flag,
//             account,
//             amount,
//             cost,
//             price,
//             metadata: metadata.unwrap_or_else(|| Metadata::new()),
//         });
//     }
// }

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Amount {
    /// The value (decimal) part
    #[pyo3(get)]
    pub number: Decimal,
    /// Currency
    #[pyo3(get)]
    pub currency: Currency,
}

// impl Into<Amount> for beancount_parser::Amount<Decimal> {
//     fn into(&self) -> Amount {
//         Amount::from(self)
//     }
// }

impl From<&beancount_parser::Amount<Decimal>> for Amount {
    fn from(value: &beancount_parser::Amount<Decimal>) -> Self {
        Amount {
            number: value.value,
            currency: value.currency.to_string(),
        }
    }
}

#[pymethods]
impl Amount {
    #[new]
    fn new(value: Decimal, currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        return Ok(Amount {
            number: value,
            currency: currency.extract()?,
        });
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Price {
    #[pyo3(get)]
    pub meta: Metadata, // PyDict
    #[pyo3(get)]
    pub date: NaiveDate, // PyDate
    #[pyo3(get)]
    pub currency: Currency,
    #[pyo3(get)]
    pub amount: Amount,
}

#[pymethods]
impl Price {
    #[new]
    fn new(meta: Metadata, date: NaiveDate, currency: String, amount: Amount) -> Self {
        Price {
            meta,
            date,
            currency,
            amount,
        }
    }
}

#[allow(deprecated)]
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq)]
pub enum Booking {
    STRICT,
    #[allow(non_camel_case_types)]
    STRICT_WITH_SIZE,
    None,
    AVERAGE,
    FIFO,
    LIFO,
    HIFO,
}

#[pymethods]
impl Booking {
    // make it behave like enum.Enum
    #[getter]
    fn name(&self) -> &str {
        return self.__str__();
    }

    #[getter]
    fn value(&self) -> &str {
        return self.__str__();
    }

    // to support both `Booking.STRICT == Booking.STRICT` and `Booking.STRICT == "STRICT"`
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(s) = other.downcast::<PyString>() {
            return s.to_cow().map(|rhs| self.__str__() == rhs);
        }

        if let Ok(b) = other.downcast::<Self>() {
            return Ok(self == b.get());
        }

        return Ok(false);
    }

    fn __str__(&self) -> &str {
        match self {
            Booking::STRICT => "STRICT",
            Booking::STRICT_WITH_SIZE => "STRICT_WITH_SIZE",
            Booking::None => "None",
            Booking::AVERAGE => "AVERAGE",
            Booking::FIFO => "FIFO",
            Booking::LIFO => "LIFO",
            Booking::HIFO => "HIFO",
        }
    }
}

// #[pymethods]
// impl Cost {
//     #[new]
//     #[pyo3(signature = (meta, date, currency, label=None))]
//     fn new(
//         meta: Metadata,
//         date: NaiveDate,
//         currency: &Bound<'_, PyString>,
//         label: Option<String>,
//     ) -> PyResult<Self> {
//         return Ok(Cost {
//             date,
//             currency: currency.extract()?,
//             label,
//         });
//     }
// }

#[pyclass]
#[derive(Debug, Clone)]
pub struct Event {
    pub meta: Metadata,  // PyDict
    pub date: NaiveDate, // PyDate
    pub typ: String,
    pub description: String,
}
