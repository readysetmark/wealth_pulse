use rust_core::str::FromStr;
use chomp::{Input, U8Result};
use chomp::{count, option, or, string, take_while, take_while1, token};
use chomp::ascii::{digit, is_digit, is_end_of_line, is_horizontal_space};
use chomp::buffer::{Source, Stream, StreamError};
use chrono::date::Date;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use decimal::d128;
use std::fs::File;
use std::str;
use core::instrument::*;
use core::price::*;
use core::symbol::*;


// HELPERS

fn to_i32(slice: Vec<u8>) -> i32 {
    // TODO: make "safe" -- ensure all u8's are actually "digits"
    slice.iter().fold(0,
        |acc, &d| (acc * 10) + ((d - ('0' as u8)) as i32))
}

fn to_u32(slice: Vec<u8>) -> u32 {
    // TODO: make "safe" -- ensure all u8's are actually "digits"
    slice.iter().fold(0u32,
        |acc, &d| (acc * 10u32) + ((d - ('0' as u8)) as u32))
}

fn make_amount(sign: u8, number: &[u8]) -> d128 {
    let mut qty = String::new();
    if sign == b'-' {
        qty.push_str(str::from_utf8(&[sign]).unwrap());
    }
    qty.push_str(str::from_utf8(number).unwrap());
    qty = qty.replace(",", "");
    d128::from_str(&qty[..]).unwrap()
}

fn is_quoted_symbol_char(c: u8) -> bool {
    c != b'\"' && c != b'\r' && c != b'\n'
}

fn is_unquoted_symbol_char(c: u8) -> bool {
    c != b'-' && c != b';' && c != b'\"' && !is_end_of_line(c)
     && !is_digit(c) && !is_horizontal_space(c)
}

fn is_amount_char(c: u8) -> bool {
    is_digit(c) || c == b'.' || c == b','
}


// PARSERS

fn whitespace(i: Input<u8>) -> U8Result<Spacing> {
    take_while(i, is_horizontal_space)
        .map(|ws|
            if ws.len() > 0 { Spacing::Space }
            else { Spacing::NoSpace })
}

fn mandatory_whitespace(i: Input<u8>) -> U8Result<()> {
    take_while1(i, is_horizontal_space).map(|_| ())
}

fn line_ending(i: Input<u8>) -> U8Result<()> {
    or(i,
        |i| token(i, b'\n').map(|_| ()),
        |i| string(i, b"\r\n").map(|_| ()))
}

fn year(i: Input<u8>) -> U8Result<i32> {
    count(i, 4, |i| digit(i)).map(to_i32)
}

fn month(i: Input<u8>) -> U8Result<u32> {
    count(i, 2, |i| digit(i)).map(to_u32)
}

fn day(i: Input<u8>) -> U8Result<u32> {
    count(i, 2, |i| digit(i)).map(to_u32)
}

fn date(i: Input<u8>) -> U8Result<Date<Local>> {
    parse!{i;
        let year =  year();
        token(b'-');
        let month = month();
        token(b'-');
        let day =   day();

        ret Local.ymd(year, month, day)
    }
}

fn quoted_symbol(i: Input<u8>) -> U8Result<Symbol> {
    parse!{i;
        token(b'\"');
        let symbol = take_while1(is_quoted_symbol_char);
        token(b'\"');

        ret Symbol::new(str::from_utf8(symbol).unwrap(), QuoteOption::Quoted)
    }
}

fn unquoted_symbol(i: Input<u8>) -> U8Result<Symbol> {
    take_while1(i, is_unquoted_symbol_char)
        .map(|b| Symbol::new(str::from_utf8(b).unwrap(), QuoteOption::Unquoted))
}

fn symbol(i: Input<u8>) -> U8Result<Symbol> {
    or(i, quoted_symbol, unquoted_symbol)
}

fn amount(i: Input<u8>) -> U8Result<d128> {
    parse!{i;
        let sign = option(|i| token(i, b'-'), b'+');
        let number = take_while1(is_amount_char);
        ret make_amount(sign, number)
    }
}

fn instrument_symbol_then_amount(i: Input<u8>) -> U8Result<Instrument> {
    parse!{i;
        let symbol = symbol();
        let spacing = whitespace();
        let amount = amount();

        ret Instrument::new(amount, symbol,
            RenderOptions::new(SymbolPosition::Left, spacing))
    }
}

fn instrument_amount_then_symbol(i: Input<u8>) -> U8Result<Instrument> {
    parse!{i;
        let amount = amount();
        let spacing = whitespace();
        let symbol = symbol();

        ret Instrument::new(amount, symbol,
            RenderOptions::new(SymbolPosition::Right, spacing))
    }
}

fn instrument(i: Input<u8>) -> U8Result<Instrument> {
    or(i, instrument_symbol_then_amount, instrument_amount_then_symbol)
}

fn price(i: Input<u8>) -> U8Result<Price> {
    parse!{i;
        token(b'P');
        mandatory_whitespace();
        let date = date();
        mandatory_whitespace();
        let symbol = symbol();
        mandatory_whitespace();
        let instrument = instrument();

        ret Price::new(date, symbol, instrument)
    }
}

fn price_line(i: Input<u8>) -> U8Result<Price> {
    parse!{i;
        let price = price();
        line_ending();
        ret price
    }
}


// FILES

pub fn parse_pricedb(file_path: &str) -> Vec<Price> {
    println!("Using chomp");
    let file = File::open(file_path).ok().expect("Failed to open file");
    let mut source = Source::new(file);

    let mut prices: Vec<Price> = Vec::new();

    loop {
        match source.parse(price_line) {
            Ok(price)                    => prices.push(price),
            Err(StreamError::Retry)      => {}, // Needed to refill buffer
            Err(StreamError::EndOfInput) => break,
            Err(e)                       => panic!("{:?}", e),
        }
    }

    prices
}



#[cfg(test)]
mod tests {
    use super::{date, day, instrument, instrument_amount_then_symbol,
        instrument_symbol_then_amount, make_amount, month, price, parse_pricedb, price_line,
        amount, quoted_symbol, unquoted_symbol, symbol, whitespace, year};
    use chomp::{parse_only};
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;
    use core::instrument::*;
    use core::price::*;
    use core::symbol::*;

    // HELPERS

    #[test]
    fn make_amount_positive_value() {
        let qty = make_amount(b'+', b"5,241.51");
        assert_eq!(qty, d128!(5241.51));
    }

    #[test]
    fn make_amount_negative_value() {
        let qty = make_amount(b'-', b"5,241.51");
        assert_eq!(qty, d128!(-5241.51));
    }


    // PARSERS

    #[test]
    fn whitespace_space() {
        let result = parse_only(whitespace, b" ");
        assert_eq!(result, Ok(Spacing::Space));
    }

    #[test]
    fn whitespace_tab() {
        let result = parse_only(whitespace, b"\t");
        assert_eq!(result, Ok(Spacing::Space));
    }

    #[test]
    fn whitespace_empty() {
        let result = parse_only(whitespace, b"");
        assert_eq!(result, Ok(Spacing::NoSpace));
    }

    #[test]
    fn year_valid() {
        let result = parse_only(year, b"2016");
        assert_eq!(result, Ok(2016));
    }

    #[test]
    fn month_valid() {
        let result = parse_only(month, b"02");
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn day_valid() {
        let result = parse_only(day, b"07");
        assert_eq!(result, Ok(7));
    }

    #[test]
    fn date_valid() {
        let result = parse_only(date, b"2016-02-07");
        assert_eq!(result, Ok(Local.ymd(2016, 2, 7)));
    }

    #[test]
    fn quoted_symbol_valid() {
        let result = parse_only(quoted_symbol, b"\"MUTF2351\"");
        assert_eq!(result, Ok(Symbol::new("MUTF2351", QuoteOption::Quoted)));
    }

    #[test]
    fn unquoted_symbol_just_symbol() {
        let result = parse_only(unquoted_symbol, b"$");
        assert_eq!(result, Ok(Symbol::new("$", QuoteOption::Unquoted)));
    }

    #[test]
    fn unquoted_symbol_symbol_and_letters() {
        let result = parse_only(unquoted_symbol, b"US$");
        assert_eq!(result, Ok(Symbol::new("US$", QuoteOption::Unquoted)));
    }

    #[test]
    fn unquoted_symbol_just_letters() {
        let result = parse_only(unquoted_symbol, b"RUST");
        assert_eq!(result, Ok(Symbol::new("RUST", QuoteOption::Unquoted)));
    }

    #[test]
    fn symbol_quoted() {
        let result = parse_only(symbol, b"\"MUTF2351\"");
        assert_eq!(result, Ok(Symbol::new("MUTF2351", QuoteOption::Quoted)));
    }

    #[test]
    fn symbol_unquoted() {
        let result = parse_only(symbol, b"$");
        assert_eq!(result, Ok(Symbol::new("$", QuoteOption::Unquoted)));
    }

    #[test]
    fn amount_negative_no_fractional_part() {
        let result = parse_only(amount, b"-1110");
        assert_eq!(result, Ok(d128!(-1110)));
    }

    #[test]
    fn amount_positive_no_fractional_part() {
        let result = parse_only(amount, b"2,314");
        assert_eq!(result, Ok(d128!(2314)));
    }

    #[test]
    fn amount_negative_with_fractional_part() {
        let result = parse_only(amount, b"-1,110.38");
        assert_eq!(result, Ok(d128!(-1110.38)));
    }

    #[test]
    fn amount_positive_with_fractional_part() {
        let result = parse_only(amount, b"2314.793");
        assert_eq!(result, Ok(d128!(2314.793)));
    }

    #[test]
    fn instrument_symbol_then_amount_no_whitespace() {
        let result = parse_only(instrument_symbol_then_amount, b"$13,245.00");
        assert_eq!(result, Ok(Instrument::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
        )));
    }

    #[test]
    fn instrument_symbol_then_amount_with_whitespace() {
        let result = parse_only(instrument_symbol_then_amount, b"US$ -13,245.00");
        assert_eq!(result, Ok(Instrument::new(
            d128!(-13245.00),
            Symbol::new("US$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::Space)
        )));
    }

    #[test]
    fn instrument_amount_then_symbol_no_whitespace() {
        let result = parse_only(instrument_amount_then_symbol, b"13,245.463RUST");
        assert_eq!(result, Ok(Instrument::new(
            d128!(13245.463),
            Symbol::new("RUST", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::NoSpace)
        )));
    }

    #[test]
    fn instrument_amount_then_symbol_with_whitespace() {
        let result = parse_only(instrument_amount_then_symbol,
            b"13,245.463 \"MUTF2351\"");
        assert_eq!(result, Ok(Instrument::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space)
        )));
    }    

    #[test]
    fn instrument_with_symbol_then_amount() {
        let result = parse_only(instrument, b"$13,245.46");
        assert_eq!(result, Ok(Instrument::new(
            d128!(13245.46),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
        )));
    }

    #[test]
    fn instrument_with_amount_then_symbol() {
        let result = parse_only(instrument, b"13,245.463 \"MUTF2351\"");
        assert_eq!(result, Ok(Instrument::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space)
        )));
    }

    #[test]
    fn price_valid() {
        let result = parse_only(price, b"P 2016-02-07 \"MUTF2351\" $5.42");
        assert_eq!(result, Ok(Price::new(
            Local.ymd(2016, 2, 7),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            Instrument::new(
                d128!(5.42),
                Symbol::new("$", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
            )
        )));
    }

    #[test]
    fn price_line_valid() {
        let result = parse_only(price_line,
            b"P 2016-02-07 \"MUTF2351\" $5.42\r\n");
        assert_eq!(result, Ok(Price::new(
            Local.ymd(2016, 2, 7),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            Instrument::new(
                d128!(5.42),
                Symbol::new("$", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
            )
        )));
    }

    #[test]
    fn pricedb_empty() {
        let result = parse_pricedb("./test/data/empty.pricedb");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn pricedb_single() {
        let result = parse_pricedb("./test/data/single.pricedb");
        assert_eq!(result, vec![
            Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Instrument::new(
                    d128!(5.41),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
        ]);
    }

    #[test]
    fn pricedb_multiple() {
        let result = parse_pricedb("./test/data/multiple.pricedb");
        assert_eq!(result, vec![
            Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Instrument::new(
                    d128!(5.41),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
            Price::new(
                Local.ymd(2016, 2, 8),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Instrument::new(
                    d128!(5.61),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
            Price::new(
                Local.ymd(2016, 2, 9),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Instrument::new(
                    d128!(7.10),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
        ]);
    }
}