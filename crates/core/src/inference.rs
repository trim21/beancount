use std::collections::HashMap;

use rust_decimal::Decimal;

use beancount_parser::core::{
  number_expr_to_decimal, Amount, CoreDirective, CostAmount, CostSpec, NumberExpr, Posting,
  Transaction,
};
use beancount_parser::{ast, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredDirective {
  Transaction(InferredTransaction),
  Other(CoreDirective),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredTransaction {
  pub meta: ast::Meta,
  pub span: ast::Span,
  pub date: String,
  pub txn: Option<String>,
  pub payee: Option<String>,
  pub narration: Option<String>,
  pub tags: beancount_parser::core::SmallStrVec,
  pub links: beancount_parser::core::SmallStrVec,
  pub key_values: beancount_parser::core::SmallKeyValues,
  pub postings: Vec<InferredPosting>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredPosting {
  pub meta: ast::Meta,
  pub span: ast::Span,
  pub opt_flag: Option<String>,
  pub account: String,
  pub amount: InferredAmount,
  pub cost_spec: Option<beancount_parser::core::CostSpec>,
  pub price_operator: Option<ast::PriceOperator>,
  pub price_annotation: Option<InferredAmount>,
  pub comment: Option<String>,
  pub key_values: beancount_parser::core::SmallKeyValues,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredAmount {
  pub raw: String,
  pub number: Decimal,
  pub currency: String,
}

#[derive(Default)]
struct CurrencyState {
  sum: Decimal,
  missing_indices: Vec<usize>,
}

fn resolve_amount(amount: &Amount, meta: &ast::Meta) -> Result<InferredAmount, ParseError> {
  if matches!(amount.number, NumberExpr::Missing) {
    return Err(ParseError {
      line: meta.line,
      column: meta.column,
      message: "posting amount missing after inference".to_string(),
    });
  }

  let dec = number_expr_to_decimal(&amount.number).map_err(|err| ParseError {
    line: meta.line,
    column: meta.column,
    message: err.message,
  })?;
  let normalized = dec.normalize();
  let currency = amount.currency.clone().ok_or_else(|| ParseError {
    line: meta.line,
    column: meta.column,
    message: "posting amount is missing a currency".to_string(),
  })?;

  Ok(InferredAmount {
    raw: normalized.to_string(),
    number: normalized,
    currency,
  })
}

fn resolve_cost_amount(amount: &CostAmount, meta: &ast::Meta) -> Result<CostAmount, ParseError> {
  let per = amount
    .per
    .as_ref()
    .map(|n| match n {
      NumberExpr::Missing => Ok(NumberExpr::Missing),
      _ => number_expr_to_decimal(n)
        .map(|d| NumberExpr::Literal(d.normalize().to_string()))
        .map_err(|err| ParseError {
          line: meta.line,
          column: meta.column,
          message: err.message,
        }),
    })
    .transpose()?;
  let total = amount
    .total
    .as_ref()
    .map(|n| match n {
      NumberExpr::Missing => Ok(NumberExpr::Missing),
      _ => number_expr_to_decimal(n)
        .map(|d| NumberExpr::Literal(d.normalize().to_string()))
        .map_err(|err| ParseError {
          line: meta.line,
          column: meta.column,
          message: err.message,
        }),
    })
    .transpose()?;

  Ok(CostAmount {
    per,
    total,
    currency: amount.currency.clone(),
  })
}

fn resolve_cost_spec(cost_spec: &CostSpec, meta: &ast::Meta) -> Result<CostSpec, ParseError> {
  let amount = cost_spec
    .amount
    .as_ref()
    .map(|a| resolve_cost_amount(a, meta))
    .transpose()?;

  Ok(CostSpec {
    amount,
    raw: cost_spec.raw.clone(),
    date: cost_spec.date.clone(),
    label: cost_spec.label.clone(),
    merge: cost_spec.merge,
    is_total: cost_spec.is_total,
  })
}

fn resolve_price(
  price: &Option<Amount>,
  meta: &ast::Meta,
) -> Result<Option<InferredAmount>, ParseError> {
  match price {
    None => Ok(None),
    Some(amount) => {
      if matches!(amount.number, NumberExpr::Missing) {
        Err(ParseError {
          line: meta.line,
          column: meta.column,
          message: "price number missing after inference".to_string(),
        })
      } else {
        Ok(Some(resolve_amount(amount, meta)?))
      }
    }
  }
}

/// Infer missing posting amounts for each transaction while preserving other directives.
///
/// For each currency within a transaction, if exactly one posting amount is missing
/// we balance that currency by assigning the opposite of the running total.
/// If multiple postings are missing for the same currency or the transaction cannot
/// balance, a `ParseError` is returned.
pub fn infer_directives(
  directives: Vec<CoreDirective>,
) -> Result<Vec<InferredDirective>, ParseError> {
  directives
    .into_iter()
    .map(|directive| match directive {
      CoreDirective::Transaction(txn) => {
        infer_transaction_postings(txn).map(InferredDirective::Transaction)
      }
      other => Ok(InferredDirective::Other(other)),
    })
    .collect()
}

pub fn infer_transaction_postings(
  mut txn: Transaction,
) -> Result<InferredTransaction, ParseError> {
  let mut currencies: HashMap<String, CurrencyState> = HashMap::new();

  for (idx, posting) in txn.postings.iter().enumerate() {
    match &posting.amount {
      Some(amount) => {
        let currency = amount.currency.clone().ok_or_else(|| ParseError {
          line: posting.meta.line,
          column: posting.meta.column,
          message: "posting amount is missing a currency".to_string(),
        })?;

        match &amount.number {
          NumberExpr::Missing => {
            currencies
              .entry(currency)
              .or_default()
              .missing_indices
              .push(idx);
          }
          _ => {
            let value = number_expr_to_decimal(&amount.number).map_err(|err| ParseError {
              line: posting.meta.line,
              column: posting.meta.column,
              message: err.message,
            })?;

            currencies.entry(currency).or_default().sum += value;
          }
        }
      }
      None => {
        return Err(ParseError {
          line: posting.meta.line,
          column: posting.meta.column,
          message: "posting is missing an amount; cannot infer without a currency".to_string(),
        });
      }
    }
  }

  for (currency, state) in currencies {
    if state.missing_indices.is_empty() {
      if !state.sum.is_zero() {
        return Err(ParseError {
          line: txn.meta.line,
          column: txn.meta.column,
          message: format!(
            "transaction is not balanced for currency {}: residual {}",
            currency,
            state.sum,
          ),
        });
      }
      continue;
    }

    if state.missing_indices.len() > 1 {
      return Err(ParseError {
        line: txn.meta.line,
        column: txn.meta.column,
        message: format!(
          "cannot infer amounts: {} postings are missing in currency {currency}",
          state.missing_indices.len()
        ),
      });
    }

    let missing_idx = state.missing_indices[0];
    let inferred = (-state.sum).normalize();
    let amount_str = inferred.to_string();

    let target: &mut Posting = txn
      .postings
      .get_mut(missing_idx)
      .expect("missing index recorded during scan");

    target.amount = Some(Amount {
      raw: amount_str.clone(),
      number: NumberExpr::Literal(amount_str),
      currency: Some(currency),
    });
  }

  let postings = txn
    .postings
    .into_iter()
    .map(|p| match p.amount {
      Some(amount) if !matches!(amount.number, NumberExpr::Missing) => {
        let amount = resolve_amount(&amount, &p.meta)?;
        let cost_spec = p
          .cost_spec
          .as_ref()
          .map(|c| resolve_cost_spec(c, &p.meta))
          .transpose()?;
        let price_annotation = resolve_price(&p.price_annotation, &p.meta)?;

        Ok(InferredPosting {
          meta: p.meta,
          span: p.span,
          opt_flag: p.opt_flag,
          account: p.account,
          amount,
          cost_spec,
          price_operator: p.price_operator,
          price_annotation,
          comment: p.comment,
          key_values: p.key_values,
        })
      }
      _ => Err(ParseError {
        line: p.meta.line,
        column: p.meta.column,
        message: "posting amount missing after inference".to_string(),
      }),
    })
    .collect::<Result<Vec<_>, _>>()?;

  Ok(InferredTransaction {
    meta: txn.meta,
    span: txn.span,
    date: txn.date,
    txn: txn.txn,
    payee: txn.payee,
    narration: txn.narration,
    tags: txn.tags,
    links: txn.links,
    key_values: txn.key_values,
    postings,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use beancount_parser::core::{Amount, CoreDirective, CostAmount, CostSpec, NumberExpr, Posting};
  use std::sync::Arc;

  fn meta() -> ast::Meta {
    ast::Meta {
      filename: Arc::new("test.beancount".to_string()),
      ..Default::default()
    }
  }

  fn span() -> ast::Span {
    ast::Span::default()
  }

  fn posting(amount: Amount) -> Posting {
    Posting {
      meta: meta(),
      span: span(),
      account: "Assets:Cash".to_string(),
      amount: Some(amount),
      ..Default::default()
    }
  }

  fn literal_amount(raw: &str, currency: &str) -> Amount {
    Amount {
      raw: raw.to_string(),
      number: NumberExpr::Literal(raw.to_string()),
      currency: Some(currency.to_string()),
    }
  }

  fn txn_with_postings(postings: Vec<Posting>) -> CoreDirective {
    CoreDirective::Transaction(Transaction {
      meta: meta(),
      span: span(),
      date: "2024-01-01".to_string(),
      txn: Some("*".to_string()),
      tags: Default::default(),
      links: Default::default(),
      postings: postings.into(),
      ..Default::default()
    })
  }

  #[test]
  fn infers_single_missing_amount() {
    let missing = Amount {
      raw: "".to_string(),
      number: NumberExpr::Missing,
      currency: Some("USD".to_string()),
    };
    let present = Amount {
      raw: "10".to_string(),
      number: NumberExpr::Literal("10".to_string()),
      currency: Some("USD".to_string()),
    };

    let directives = vec![txn_with_postings(vec![posting(missing), posting(present)])];
    let inferred = infer_directives(directives).expect("inference should succeed");

    match &inferred[0] {
      InferredDirective::Transaction(txn) => {
        assert_eq!(txn.postings.len(), 2);
        let inferred_amt = &txn.postings[0].amount;
        assert_eq!(inferred_amt.currency, "USD");
        assert_eq!(inferred_amt.number, Decimal::new(-10, 0));
        assert_eq!(inferred_amt.raw, "-10");
      }
      _ => panic!("expected transaction"),
    }
  }

  #[test]
  fn fails_on_multiple_missing_same_currency() {
    let missing_usd = Amount {
      raw: "".to_string(),
      number: NumberExpr::Missing,
      currency: Some("USD".to_string()),
    };

    let directives = vec![txn_with_postings(vec![posting(missing_usd.clone()), posting(missing_usd)])];
    let err = infer_directives(directives).expect_err("should fail with two missing amounts");
    assert!(err.message.contains("cannot infer amounts"));
  }

  #[test]
  fn infers_missing_per_currency_independently() {
    let usd_missing = Amount {
      raw: "".to_string(),
      number: NumberExpr::Missing,
      currency: Some("USD".to_string()),
    };
    let usd_present = literal_amount("5", "USD");
    let eur_present = literal_amount("7", "EUR");
    let eur_missing = Amount {
      raw: "".to_string(),
      number: NumberExpr::Missing,
      currency: Some("EUR".to_string()),
    };

    let directives = vec![txn_with_postings(vec![
      posting(usd_missing),
      posting(usd_present),
      posting(eur_present),
      posting(eur_missing),
    ])];

    let inferred = infer_directives(directives).expect("inference should succeed");
    let InferredDirective::Transaction(txn) = &inferred[0] else {
      panic!("expected transaction");
    };

    assert_eq!(txn.postings.len(), 4);
    let usd_inferred = &txn.postings[0].amount;
    assert_eq!(usd_inferred.currency, "USD");
    assert_eq!(usd_inferred.number, Decimal::new(-5, 0));

    let eur_inferred = &txn.postings[3].amount;
    assert_eq!(eur_inferred.currency, "EUR");
    assert_eq!(eur_inferred.number, Decimal::new(-7, 0));
  }

  #[test]
  fn normalizes_cost_spec_numbers() {
    let amt = literal_amount("1", "USD");
    let cost_amount = CostAmount {
      per: Some(NumberExpr::Literal("01.2300".to_string())),
      total: Some(NumberExpr::Literal("2_0".to_string())),
      currency: Some("USD".to_string()),
    };
    let cost_spec = CostSpec {
      amount: Some(cost_amount),
      raw: String::new(),
      ..Default::default()
    };

    let mut p = posting(amt);
    p.cost_spec = Some(cost_spec);

    let balancing = posting(literal_amount("-1", "USD"));
    let directives = vec![txn_with_postings(vec![p, balancing])];
    let inferred = infer_directives(directives).expect("inference should succeed");
    let InferredDirective::Transaction(txn) = &inferred[0] else {
      panic!("expected transaction");
    };
    let cost_spec = txn.postings[0].cost_spec.as_ref().expect("cost spec present");
    let amount = cost_spec.amount.as_ref().expect("cost amount present");

    assert!(matches!(amount.per, Some(NumberExpr::Literal(ref n)) if n == "1.23"));
    assert!(matches!(amount.total, Some(NumberExpr::Literal(ref n)) if n == "20"));
  }

  #[test]
  fn normalizes_price_annotation() {
    let amt = literal_amount("10", "USD");
    let price = literal_amount("2.500", "CAD");

    let mut p = posting(amt);
    p.price_annotation = Some(price);

    let balancing = posting(literal_amount("-10", "USD"));
    let directives = vec![txn_with_postings(vec![p, balancing])];
    let inferred = infer_directives(directives).expect("inference should succeed");
    let InferredDirective::Transaction(txn) = &inferred[0] else {
      panic!("expected transaction");
    };
    let price = txn.postings[0]
      .price_annotation
      .as_ref()
      .expect("price annotation present");

    assert_eq!(price.currency, "CAD");
    assert_eq!(price.number, Decimal::new(25, 1));
    assert_eq!(price.raw, "2.5");
  }
}
