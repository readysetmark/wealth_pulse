use core::amount::*;
use core::price::Price;
use core::transaction::*;

#[derive(PartialEq, Debug)]
pub enum AmountSource {
    Provided,
    Inferred,
}

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
    Transaction(Header, Vec<RawPosting>),
}