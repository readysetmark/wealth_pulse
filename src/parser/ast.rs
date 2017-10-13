use core::amount::*;
use core::posting::*;
use core::price::Price;
use core::header::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
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

    /// Validate and transform header into a vector of `Posting`s. This means:
    /// - autobalance transactions: transaction can be missing 1 amount, which we can infer
    /// - ensure transactions balance: transaction must balance to 0
    /// - transform to vector of postings
    pub fn validate_and_transform(&self) -> Result<(), String> {
        let status = calculate_balance_status(&self.postings);

        match status {
            TransactionBalanceStatus::Balanced => {
                Result::Ok(())
            },
            TransactionBalanceStatus::InferredAmount{
                posting_index, amount
            } => {
                Result::Ok(())
            },
            TransactionBalanceStatus::MultipleInferredAmounts(num_inferred) => {
                // TODO: better error message! start with line number, but a fancy
                // error message would be better!
                Result::Err(format!("Encountered {} inferred amounts", num_inferred))
            },
            TransactionBalanceStatus::Unbalanced(_) => {
                // TODO: as above, better error message!
                Result::Err("Encountered unbalanced transaction".to_string())
            },
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ParseTree {
    Price(Price),
    Transaction(RawTransaction),
}

#[derive(PartialEq, Debug)]
enum TransactionBalanceStatus {
    Balanced,
    InferredAmount { posting_index: usize, amount: Amount },
    MultipleInferredAmounts(u32),
    Unbalanced(HashMap<String, Amount>),
}

/// Calculate balance status
/// All symbols must balance to 0, or at most one entry can be missing an amount, which
/// we can infer to be the remaining balance.
fn calculate_balance_status(postings: & Vec<RawPosting>) -> TransactionBalanceStatus {
    let mut balance: HashMap<String, Amount> = HashMap::new();
    let mut num_inferred_amounts = 0;
    let mut inferred_posting_index = 0;

    for (index, posting) in postings.iter().enumerate() {
        match posting.amount {
            Some(ref amount) => {
                match balance.entry(amount.symbol.value.clone()) {
                    Entry::Occupied(mut e) => {
                        let mut value = e.get_mut();
                        value.quantity += amount.quantity;
                    },
                    Entry::Vacant(e) => {
                        e.insert(amount.clone());
                    },
                };
            },
            None => {
                num_inferred_amounts += 1;
                inferred_posting_index = index;
            },
        };
    }

    let unbalanced_symbols: HashMap<String, Amount> = balance.into_iter()
        .filter(|&(_, ref amount)| amount.quantity != d128!(0))
        .collect();

    if num_inferred_amounts > 1 {
        TransactionBalanceStatus::MultipleInferredAmounts(num_inferred_amounts)
    }
    else if num_inferred_amounts == 1 && unbalanced_symbols.len() == 1 {
        let (_, remaining_balance) = unbalanced_symbols.iter().nth(0).unwrap();

        TransactionBalanceStatus::InferredAmount {
            posting_index: inferred_posting_index,
            amount: Amount::new(
                d128!(-1) * remaining_balance.quantity,
                remaining_balance.symbol.clone(),
                remaining_balance.render_options.clone()
            ),
        }
    }
    else if unbalanced_symbols.len() > 0 {
        TransactionBalanceStatus::Unbalanced(unbalanced_symbols)
    }
    else {
        TransactionBalanceStatus::Balanced
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::symbol::*;

    #[test]
    fn calculate_balance_status_balanced_no_postings() {
        let v: Vec<RawPosting> = Vec::new();
        assert_eq!(calculate_balance_status(&v), TransactionBalanceStatus::Balanced);
    }

    #[test]
    fn calculate_balance_status_balanced_no_inferred() {
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
        assert_eq!(calculate_balance_status(&v), TransactionBalanceStatus::Balanced);
    }

    #[test]
    fn calculate_balance_status_unbalanced_no_inferred() {
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
        let result = calculate_balance_status(&v);
        assert_eq!(result, TransactionBalanceStatus::Unbalanced(expected_balances));
    }

    #[test]
    fn calculate_balance_status_one_inferred() {
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
        let expected = TransactionBalanceStatus::InferredAmount {
            posting_index: 3,
            amount: Amount::new(
                d128!(15.27),
                Symbol::new("MUTF2394", QuoteOption::Quoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::Space)
            ),
        };
        let result = calculate_balance_status(&v);
        assert_eq!(result, expected);
    }

    #[test]
    fn calculate_balance_status_all_inferred() {
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
        assert_eq!(calculate_balance_status(&v), TransactionBalanceStatus::MultipleInferredAmounts(4));
    }
}