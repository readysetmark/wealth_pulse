use chrono::date::Date;
use chrono::offset::local::Local;
use std::fmt;
use super::amount::Amount;
use super::symbol::Symbol;

#[derive(PartialEq, Debug)]
pub struct Price {
    // TODO: add line field
    date: Date<Local>,
    symbol: Symbol,
    amount: Amount
}

impl Price {
    pub fn new(date: Date<Local>, symbol: Symbol, amount: Amount) -> Price {
        Price {
            date: date,
            symbol: symbol,
            amount: amount
        }
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P {} {} {}",
            self.date.format("%Y-%m-%d"),
            self.symbol,
            self.amount)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::amount::*;
    use core::symbol::*;
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;

    #[test]
    fn price_fmt() {
        let result =
            format!("{}", Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    AmountRenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))));
        assert_eq!(result, "P 2016-02-07 \"MUTF2351\" $5.42");
    }
}