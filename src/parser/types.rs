use core::commodity::*;
use core::price::Price;
use core::transaction::*;

#[derive(PartialEq, Debug)]
pub enum CommoditySource {
    Provided,
    Inferred,
}

#[derive(PartialEq, Debug)]
pub struct RawPosting {
    full_account: String,
    sub_accounts: Vec<String>,
    commodity: Option<Commodity>,
    commodity_source: CommoditySource,
    comment: Option<String>,
}

impl RawPosting {
    pub fn new(sub_accounts: Vec<String>, commodity: Option<Commodity>,
    commodity_source: CommoditySource, comment: Option<String>) -> RawPosting {
        RawPosting {
            full_account: sub_accounts.join(":"),
            sub_accounts: sub_accounts,
            commodity: commodity,
            commodity_source: commodity_source,
            comment: comment
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ParseTree {
    Price(Price),
    Transaction(Header, Vec<RawPosting>),
}