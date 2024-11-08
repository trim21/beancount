use crate::decimal::Decimal;
use chrono::NaiveDate;
#[allow(unused_imports)]
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyString};
use pyo3::{pyclass, pymethods, Bound, PyAny, PyResult};
use std::collections::HashMap;
use std::fmt::Display;

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
            self.meta, self.date, self.account, self.currencies, self.booking.as_ref().map(|x| x.__str__())
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
        return format!("Close(meta={:?}, date={}, account={})", self.meta, self.date, self.account);
    }

    fn __repr__(&self) -> String {
        return self.__str__();
    }
}


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

#[pymethods]
impl Amount {
    #[new]
    fn new(value: &Bound<'_, PyAny>, currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        return Ok(Amount {
            number: Decimal::from_py(value)?,
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
    fn new(
        meta: Metadata,
        date: NaiveDate,
        currency: String,
        amount: Amount,
    ) -> Self {
        Price {
            meta,
            date,
            currency,
            amount,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum PostingPrice {
    /// Unit cost (`@`)
    Unit(Amount),
    /// Total cost (`@@`)
    Total(Amount),
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

#[pyclass]
#[derive(Debug, Clone)]
pub struct Cost {
    #[pyo3(get)]
    pub meta: Metadata, // PyDict
    #[pyo3(get)]
    pub date: NaiveDate, // PyDate
    #[pyo3(get)]
    pub currency: Currency,
    pub label: Option<String>,
}

#[pymethods]
impl Cost {
    #[new]
    #[pyo3(signature = (meta, date, currency, label=None))]
    fn new(
        meta: Metadata,
        date: NaiveDate,
        currency: &Bound<'_, PyString>,
        label: Option<String>,
    ) -> PyResult<Self> {
        return Ok(Cost {
            meta,
            date,
            currency: currency.extract()?,
            label,
        });
    }
}

// #[derive(Debug, Clone)]
// #[non_exhaustive]
#[pyclass]
#[derive(Debug, Clone)]
pub struct Posting {
    /// Transaction flag (`*` or `!` or `None` when absent)
    pub flag: Option<char>,
    /// Account modified by the posting
    pub account: String,
    /// Amount being added to the account
    pub amount: Option<Amount>,
    /// Cost (content within `{` and `}`)
    pub cost: Option<Cost>,
    /// Price (`@` or `@@`) syntax
    pub price: Option<PostingPrice>,
    /// The metadata attached to the posting
    pub metadata: Metadata,
}

#[pymethods]
impl Posting {
    #[new]
    #[pyo3(signature = (flag, account, amount=None, cost=None, price=None, metadata=None))]
    fn new(
        flag: Option<char>,
        account: String,
        amount: Option<Amount>,
        cost: Option<Cost>,
        price: Option<PostingPrice>,
        metadata: Option<Metadata>,
    ) -> PyResult<Self> {
        return Ok(Posting {
            flag,
            account,
            amount,
            cost,
            price,
            metadata: metadata.unwrap_or_else(|| Metadata::new()),
        });
    }
}
