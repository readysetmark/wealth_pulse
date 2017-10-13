use std::rc::Rc;
use super::amount::Amount;
use super::header::Header;

#[derive(PartialEq, Debug)]
pub enum AmountSource {
    Provided,
    Inferred,
}

#[derive(PartialEq, Debug)]
pub struct Posting {
    header: Rc<Header>,
    account: String,
    account_lineage: Vec<String>, 
    amount: Amount,
    amount_source: AmountSource,
    comment: Option<String>,
}

impl Posting {
    pub fn new(header: Rc<Header>, account: String,
    account_lineage: Vec<String>, amount: Amount, amount_source: AmountSource,
    comment: Option<String>) -> Posting {
        Posting {
            header,
            account,
            account_lineage,
            amount,
            amount_source,
            comment
        }
    }
}
