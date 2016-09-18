use core::instrument::*;
use core::price::Price;
use core::transaction::*;

#[derive(PartialEq, Debug)]
pub enum InstrumentSource {
    Provided,
    Inferred,
}

#[derive(PartialEq, Debug)]
pub struct RawPosting {
    full_account: String,
    sub_accounts: Vec<String>,
    instrument: Option<Instrument>,
    instrument_source: InstrumentSource,
    comment: Option<String>,
}

impl RawPosting {
    pub fn new(sub_accounts: Vec<String>, instrument: Option<Instrument>,
    instrument_source: InstrumentSource, comment: Option<String>) -> RawPosting {
        RawPosting {
            full_account: sub_accounts.join(":"),
            sub_accounts: sub_accounts,
            instrument: instrument,
            instrument_source: instrument_source,
            comment: comment
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ParseTree {
    Price(Price),
    Transaction(Header, Vec<RawPosting>),
}