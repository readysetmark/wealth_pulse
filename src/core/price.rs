use chrono::date::Date;
use chrono::offset::local::Local;
use std::fmt;
use super::instrument::Instrument;
use super::symbol::Symbol;

#[derive(PartialEq, Debug)]
pub struct Price {
    date: Date<Local>,
    symbol: Symbol,
    instrument: Instrument,
}

impl Price {
    pub fn new(date: Date<Local>, symbol: Symbol, instrument: Instrument) -> Price {
        Price {
            date: date,
            symbol: symbol,
            instrument: instrument,
        }
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P {} {} {}", self.date.format("%Y-%m-%d"), self.symbol, self.instrument)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::instrument::*;
    use core::symbol::*;
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;

    #[test]
    fn price_fmt() {
        let result = format!("{}", Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Instrument::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
        assert_eq!(result, "P 2016-02-07 \"MUTF2351\" $5.42");
    }
}