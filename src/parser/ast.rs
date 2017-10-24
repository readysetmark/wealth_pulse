use core::amount::*;
use core::posting::*;
use core::price::Price;
use core::header::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::result::Result;


#[derive(PartialEq, Debug)]
pub struct RawPosting {
    full_account: String,
    sub_accounts: Vec<String>,
    amount: Option<Amount>,
    amount_source: AmountSource,
    comment: Option<String>,
}

impl RawPosting {
    pub fn new(sub_accounts: Vec<String>, amount: Option<Amount>,
    amount_source: AmountSource, comment: Option<String>) -> RawPosting {
        RawPosting {
            full_account: sub_accounts.join(":"),
            sub_accounts: sub_accounts,
            amount: amount,
            amount_source: amount_source,
            comment: comment
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ParseTree {
    Price(Price),
    Transaction(RawTransaction),
}

#[derive(PartialEq, Debug)]
pub struct RawTransaction {
    header: Header,
    postings: Vec<RawPosting>
}

impl RawTransaction {
    pub fn new(header: Header, postings: Vec<RawPosting>) -> RawTransaction {
        RawTransaction {
            header: header,
            postings: postings
        }
    }

    /// Attempt to transform transaction header and raw postings into
    /// a vector of `Posting`s. Validate that transactions balance to 0,
    /// autobalance any transactions where there as a single inferred amount,
    /// and transform into `Postings` if successful.
    pub fn into_postings(self) -> Result<Vec<Posting>, String> {
        let balance_status = ensure_balanced(self.postings);

        match balance_status {
            RawTransactionBalanceStatus::Balanced(raw_postings) => {
                Ok(into_postings(self.header, raw_postings))
            }
            RawTransactionBalanceStatus::MultipleAmountsMissing(num_missing) => {
                // TODO: should provide a better error message
                Err(format!("Encountered {} missing amounts", num_missing))
            }
            RawTransactionBalanceStatus::Unbalanced(_remaining_balances) => {
                // TODO: use `remaining_balances` in the error msg
                Err(format!("Encountered unbalanced transaction"))
            }
        }
    }
}

#[derive(PartialEq, Debug)]
enum RawTransactionBalanceStatus {
    Balanced(Vec<RawPosting>),
    MultipleAmountsMissing(u32),
    Unbalanced(HashMap<String, Amount>),
}

/// Ensure the transaction is balance with respect to all amounts and symbols. If the
/// transaction is missing only 1 amount, we can infer the amount and update the `RawPosting`.
/// If more than one amount is missing, or amounts do not balance to 0, then the transaction is
/// invalid.
fn ensure_balanced(postings: Vec<RawPosting>) -> RawTransactionBalanceStatus {
    let mut balance: HashMap<String, Amount> = HashMap::new();
    let mut num_missing_amounts = 0;
    let mut inferred_posting_index = 0;

    for (index, posting) in postings.iter().enumerate() {
        match posting.amount {
            Some(ref amount) => {
                match balance.entry(amount.symbol.value.clone()) {
                    Entry::Occupied(mut e) => {
                        let value = e.get_mut();
                        value.quantity += amount.quantity;
                    },
                    Entry::Vacant(e) => {
                        e.insert(amount.clone());
                    },
                };
            },
            None => {
                num_missing_amounts += 1;
                inferred_posting_index = index;
            },
        };
    }

    let unbalanced_symbols: HashMap<String, Amount> = balance.into_iter()
        .filter(|&(_, ref amount)| amount.quantity != d128!(0))
        .collect();

    if num_missing_amounts > 1 {
        RawTransactionBalanceStatus::MultipleAmountsMissing(num_missing_amounts)
    }
    else if num_missing_amounts == 1 && unbalanced_symbols.len() == 1 {
        let (_, remaining_balance) = unbalanced_symbols.iter().nth(0).unwrap();
        let mut balanced_postings = vec!();

        for (index, posting) in postings.into_iter().enumerate() {
            if index == inferred_posting_index {
                balanced_postings.push(RawPosting::new(
                    posting.sub_accounts,
                    Some(Amount::new(
                        d128!(-1) * remaining_balance.quantity,
                        remaining_balance.symbol.clone(),
                        remaining_balance.render_options.clone()
                    )),
                    AmountSource::Inferred,
                    posting.comment
                ));
            } else {
                balanced_postings.push(posting);
            }
        }

        RawTransactionBalanceStatus::Balanced(balanced_postings)
    }
    else if unbalanced_symbols.len() == 0 {
        RawTransactionBalanceStatus::Balanced(postings)
    }
    else {
        RawTransactionBalanceStatus::Unbalanced(unbalanced_symbols)
    }
}

/// Transform a `Header` and `RawPostings` into a vector of `Posting`s.
/// All `RawPostings` are expected to have `Some(Amount)` at this point!
fn into_postings(header: Header, raw_postings: Vec<RawPosting>) -> Vec<Posting> {
    let header = Rc::new(header);

    raw_postings.into_iter().map(|p| {
        let account_lineage = build_account_lineage(&p.sub_accounts);
        Posting::new(
            header.clone(),
            p.full_account,
            account_lineage,
            p.amount.expect("Encountered unexpected missing amount"),
            p.amount_source,
            p.comment
        )
    }).collect()
}

/// Build a vector of full account names for all levels of accounts based on the
/// `sub_accounts` provided.
///
/// e.g. Given ["Assets", "Savings", "Bank"] we should get back ["Assets",
/// "Assets:Savings", "Assets:Savings:Bank"]
fn build_account_lineage(sub_accounts: &Vec<String>) -> Vec<String> {
    let mut account_lineage = Vec::new();
    let mut account = String::new();

    for sub_account in sub_accounts.iter() {
        if account.len() == 0 {
            account.push_str(sub_account);
        } else {
            account.push(':');
            account.push_str(sub_account);
        }
        account_lineage.push(account.clone());
    }

    account_lineage
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};
    use core::symbol::*;

    #[test]
    fn ensure_balanced_no_postings_is_balanced() {
        let v: Vec<RawPosting> = Vec::new();
        assert_eq!(ensure_balanced(v), RawTransactionBalanceStatus::Balanced(Vec::new()));
    }

    #[test]
    fn ensure_balanced_balanced_no_inferred() {
        let v: Vec<RawPosting> = vec![
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
        ];
        let expected = vec![
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
        ];
        assert_eq!(ensure_balanced(v), RawTransactionBalanceStatus::Balanced(expected));
    }

    #[test]
    fn ensure_balanced_unbalanced_no_inferred() {
        let v: Vec<RawPosting> = vec![
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(15.30),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
        ];
        let expected_balances: HashMap<String, Amount> = [
            (
                "MUTF2394".to_string(),
                Amount::new(
                    d128!(0.03),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space)
                )
            )
        ].iter().cloned().collect();
        let result = ensure_balanced(v);
        assert_eq!(result, RawTransactionBalanceStatus::Unbalanced(expected_balances));
    }

    #[test]
    fn ensure_balanced_one_inferred() {
        let v: Vec<RawPosting> = vec![
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                None,
                AmountSource::Inferred,
                None
            ),
        ];
        let expected = RawTransactionBalanceStatus::Balanced(vec![
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(-15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space))
                ),
                AmountSource::Provided,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                Some(Amount::new(
                    d128!(15.27),
                    Symbol::new("MUTF2394", QuoteOption::Quoted),
                    RenderOptions::new(SymbolPosition::Right, Spacing::Space)
                )),
                AmountSource::Inferred,
                None
            ),
        ]);
        let result = ensure_balanced(v);
        assert_eq!(result, expected);
    }

    #[test]
    fn ensure_balanced_all_inferred() {
        let v: Vec<RawPosting> = vec![
            RawPosting::new(
                Vec::<String>::new(),
                None,
                AmountSource::Inferred,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                None,
                AmountSource::Inferred,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                None,
                AmountSource::Inferred,
                None
            ),
            RawPosting::new(
                Vec::<String>::new(),
                None,
                AmountSource::Inferred,
                None
            ),
        ];
        assert_eq!(ensure_balanced(v), RawTransactionBalanceStatus::MultipleAmountsMissing(4));
    }

    #[test]
    fn raw_transaction_into_postings_with_inferred() {
        let h = Header::new(
            Local.ymd(2015, 10, 20),
            Status::Cleared,
            None,
            "Payee".to_string(),
            None
        );
        let rp = vec![
            RawPosting::new(
                vec!["Expenses".to_string(), "Cash".to_string()],
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                Some("Test".to_string())
            ),
            RawPosting::new(
                vec!["Assets".to_string(), "Savings".to_string(), "Bank".to_string()],
                None,
                AmountSource::Inferred,
                None
            ),
        ];
        let expected_h = Rc::new(h.clone());
        let expected = Ok(vec![
            Posting::new(
                expected_h.clone(),
                "Expenses:Cash".to_string(),
                vec!["Expenses".to_string(), "Expenses:Cash".to_string()],
                Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
                ),
                AmountSource::Provided,
                Some("Test".to_string())
            ),
            Posting::new(
                expected_h.clone(),
                "Assets:Savings:Bank".to_string(),
                vec!["Assets".to_string(), "Assets:Savings".to_string(), "Assets:Savings:Bank".to_string()],
                Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
                ),
                AmountSource::Inferred,
                None
            ),
        ]);
        let transaction = RawTransaction::new(h, rp);
        let result = transaction.into_postings();
        assert_eq!(result, expected);
    }

    #[test]
    fn private_into_postings_test() {
        let h = Header::new(
            Local.ymd(2015, 10, 20),
            Status::Cleared,
            None,
            "Payee".to_string(),
            None
        );
        let rp = vec![
            RawPosting::new(
                vec!["Expenses".to_string(), "Cash".to_string()],
                Some(Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Provided,
                Some("Test".to_string())
            ),
            RawPosting::new(
                vec!["Assets".to_string(), "Savings".to_string(), "Bank".to_string()],
                Some(Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))
                ),
                AmountSource::Inferred,
                None
            ),
        ];
        let expected_h = Rc::new(h.clone());
        let expected = vec![
            Posting::new(
                expected_h.clone(),
                "Expenses:Cash".to_string(),
                vec!["Expenses".to_string(), "Expenses:Cash".to_string()],
                Amount::new(
                    d128!(23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
                ),
                AmountSource::Provided,
                Some("Test".to_string())
            ),
            Posting::new(
                expected_h.clone(),
                "Assets:Savings:Bank".to_string(),
                vec!["Assets".to_string(), "Assets:Savings".to_string(), "Assets:Savings:Bank".to_string()],
                Amount::new(
                    d128!(-23.4),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
                ),
                AmountSource::Inferred,
                None
            ),
        ];
        let result = into_postings(h, rp);
        assert_eq!(result, expected);
    }

    #[test]
    fn build_account_lineage_should_provide_full_account_name_for_all_levels() {
        let sub_accounts = vec!["Assets".to_string(), "Savings".to_string(), "Bank".to_string()];
        let expected = vec!["Assets", "Assets:Savings", "Assets:Savings:Bank"];
        assert_eq!(build_account_lineage(&sub_accounts), expected);
    }
}