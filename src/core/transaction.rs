use chrono::date::Date;
use chrono::offset::local::Local;

#[derive(PartialEq, Debug)]
pub enum Status {
    Cleared,
    Uncleared,
}

#[derive(PartialEq, Debug)]
pub struct Header {
    date: Date<Local>,
    status: Status,
    code: Option<String>,
    payee: String,
    comment: Option<String>
}

impl Header {
    pub fn new(date: Date<Local>, status: Status, code: Option<String>, payee: String,
    comment: Option<String>) -> Header {
        Header {
            date: date,
            status: status,
            code: code,
            payee: payee,
            comment: comment
        }
    }
}