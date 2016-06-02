use chrono::date::Date;
use chrono::offset::local::Local;
use std::fmt;
use super::commodity::Commodity;
use super::symbol::Symbol;

#[derive(PartialEq, Debug)]
pub struct Price {
    date: Date<Local>,
    symbol: Symbol,
    commodity: Commodity,
}

impl Price {
    pub fn new(date: Date<Local>, symbol: Symbol, commodity: Commodity) -> Price {
        Price {
            date: date,
            symbol: symbol,
            commodity: commodity,
        }
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P {} {} {}", self.date.format("%Y-%m-%d"), self.symbol, self.commodity)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::commodity::*;
    use core::symbol::*;
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;

    #[test]
    fn price_fmt() {
        let result = format!("{}", Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Commodity::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
        assert_eq!(result, "P 2016-02-07 \"MUTF2351\" $5.42");
    }
}