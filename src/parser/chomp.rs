use rust_core::str::FromStr;
use chomp::{Input, U8Result};
use chomp::{count, option, or, string, take_while, take_while1, token};
use chomp::ascii::{digit};
use chomp::buffer::{Source, Stream, StreamError};
use chrono::date::Date;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use decimal::d128;
use std::fs::File;
use std::str;
use core::amount::*;
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

fn make_quantity(sign: u8, number: &[u8]) -> d128 {
    let mut qty = String::new();
    if sign == b'-' {
        qty.push_str(str::from_utf8(&[sign]).unwrap());
    }
    qty.push_str(str::from_utf8(number).unwrap());
    qty = qty.replace(",", "");
    d128::from_str(&qty[..]).unwrap()
}

fn is_whitespace_char(c: u8) -> bool {
    c == b'\t' || c == b' '
}

fn is_quoted_symbol_char(c: u8) -> bool {
    c != b'\"' && c != b'\r' && c != b'\n'
}

fn is_digit_char(c: u8) -> bool {
    b'0' <= c && c <= b'9'
}

fn is_newline_char(c: u8) -> bool {
    c == b'\r' && c == b'\n'
}

fn is_unquoted_symbol_char(c: u8) -> bool {
    c != b'-' && c != b';' && c != b'\"' && !is_newline_char(c)
     && !is_digit_char(c) && !is_whitespace_char(c)
}

fn is_quantity_char(c: u8) -> bool {
    is_digit_char(c) || c == b'.' || c == b','
}


// PARSERS

fn whitespace(i: Input<u8>) -> U8Result<Spacing> {
    take_while(i, is_whitespace_char)
        .map(|ws|
            if ws.len() > 0 { Spacing::Space }
            else { Spacing::NoSpace })
}

fn mandatory_whitespace(i: Input<u8>) -> U8Result<()> {
    take_while1(i, is_whitespace_char).map(|_| ())
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

fn quantity(i: Input<u8>) -> U8Result<d128> {
    parse!{i;
        let sign = option(|i| token(i, b'-'), b'+');
        let number = take_while1(is_quantity_char);      
        ret make_quantity(sign, number)
    }
}

fn amount_symbol_then_quantity(i: Input<u8>) -> U8Result<Amount> {
    parse!{i;
        let symbol = symbol();
        let spacing = whitespace();
        let quantity = quantity();

        ret Amount::new(quantity, symbol,
            AmountRenderOptions::new(SymbolPosition::Left, spacing))
    }
}

fn amount_quantity_then_symbol(i: Input<u8>) -> U8Result<Amount> {
    parse!{i;
        let quantity = quantity();
        let spacing = whitespace();
        let symbol = symbol();

        ret Amount::new(quantity, symbol,
            AmountRenderOptions::new(SymbolPosition::Right, spacing))
    }
}

fn amount(i: Input<u8>) -> U8Result<Amount> {
    or(i, amount_symbol_then_quantity, amount_quantity_then_symbol)
}

fn price(i: Input<u8>) -> U8Result<Price> {
    parse!{i;
        token(b'P');
        mandatory_whitespace();
        let date = date();
        mandatory_whitespace();
        let symbol = symbol();
        mandatory_whitespace();
        let amount = amount();

        ret Price::new(date, symbol, amount)
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

pub fn pricedb_file(file_path: &str) -> Vec<Price> {
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
    use super::{amount, amount_quantity_then_symbol,
        amount_symbol_then_quantity, date, day, make_quantity, month, price,
        pricedb_file, price_line, quantity, quoted_symbol, unquoted_symbol,
        symbol, whitespace, year};
    use chomp::{parse_only};
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;
    use core::amount::*;
    use core::price::*;
    use core::symbol::*;

    // HELPERS

    #[test]
    fn make_quantity_positive_value() {
        let qty = make_quantity(b'+', b"5,241.51");
        assert_eq!(qty, d128!(5241.51));
    }

    #[test]
    fn make_quantity_negative_value() {
        let qty = make_quantity(b'-', b"5,241.51");
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
    fn quantity_negative_no_fractional_part() {
        let result = parse_only(quantity, b"-1110");
        assert_eq!(result, Ok(d128!(-1110)));
    }

    #[test]
    fn quantity_positive_no_fractional_part() {
        let result = parse_only(quantity, b"2,314");
        assert_eq!(result, Ok(d128!(2314)));
    }

    #[test]
    fn quantity_negative_with_fractional_part() {
        let result = parse_only(quantity, b"-1,110.38");
        assert_eq!(result, Ok(d128!(-1110.38)));
    }

    #[test]
    fn quantity_positive_with_fractional_part() {
        let result = parse_only(quantity, b"2314.793");
        assert_eq!(result, Ok(d128!(2314.793)));
    }

    #[test]
    fn amount_symbol_then_quantity_no_whitespace() {
        let result = parse_only(amount_symbol_then_quantity, b"$13,245.00");
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            AmountRenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
        )));
    }

    #[test]
    fn amount_symbol_then_quantity_with_whitespace() {
        let result = parse_only(amount_symbol_then_quantity, b"US$ -13,245.00");
        assert_eq!(result, Ok(Amount::new(
            d128!(-13245.00),
            Symbol::new("US$", QuoteOption::Unquoted),
            AmountRenderOptions::new(SymbolPosition::Left, Spacing::Space)
        )));
    }

    #[test]
    fn amount_quantity_then_symbol_no_whitespace() {
        let result = parse_only(amount_quantity_then_symbol, b"13,245.463RUST");
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("RUST", QuoteOption::Unquoted),
            AmountRenderOptions::new(SymbolPosition::Right, Spacing::NoSpace)
        )));
    }

    #[test]
    fn amount_quantity_then_symbol_with_whitespace() {
        let result = parse_only(amount_quantity_then_symbol,
            b"13,245.463 \"MUTF2351\"");
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            AmountRenderOptions::new(SymbolPosition::Right, Spacing::Space)
        )));
    }    

    #[test]
    fn amount_with_symbol_then_quantity() {
        let result = parse_only(amount, b"$13,245.46");
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.46),
            Symbol::new("$", QuoteOption::Unquoted),
            AmountRenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
        )));
    }

    #[test]
    fn amount_with_quantity_then_symbol() {
        let result = parse_only(amount, b"13,245.463 \"MUTF2351\"");
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            AmountRenderOptions::new(SymbolPosition::Right, Spacing::Space)
        )));
    }

    #[test]
    fn price_valid() {
        let result = parse_only(price, b"P 2016-02-07 \"MUTF2351\" $5.42");
        assert_eq!(result, Ok(Price::new(
            Local.ymd(2016, 2, 7),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            Amount::new(
                d128!(5.42),
                Symbol::new("$", QuoteOption::Unquoted),
                AmountRenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
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
            Amount::new(
                d128!(5.42),
                Symbol::new("$", QuoteOption::Unquoted),
                AmountRenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)
            )
        )));
    }

    #[test]
    fn pricedb_empty() {
        let result = pricedb_file("./test/data/empty.pricedb");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn pricedb_single() {
        let result = pricedb_file("./test/data/single.pricedb");
        assert_eq!(result, vec![
            Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(5.41),
                    Symbol::new("$", QuoteOption::Unquoted),
                    AmountRenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
        ]);
    }

    #[test]
    fn pricedb_multiple() {
        let result = pricedb_file("./test/data/multiple.pricedb");
        assert_eq!(result, vec![
            Price::new(
                Local.ymd(2016, 2, 7),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(5.41),
                    Symbol::new("$", QuoteOption::Unquoted),
                    AmountRenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
            Price::new(
                Local.ymd(2016, 2, 8),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(5.61),
                    Symbol::new("$", QuoteOption::Unquoted),
                    AmountRenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
            Price::new(
                Local.ymd(2016, 2, 9),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(7.10),
                    Symbol::new("$", QuoteOption::Unquoted),
                    AmountRenderOptions::new(
                        SymbolPosition::Left,
                        Spacing::NoSpace))
            ),
        ]);
    }
}