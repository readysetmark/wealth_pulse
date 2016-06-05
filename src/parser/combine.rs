use rust_core::str::FromStr;
use chrono::date::Date;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use combine::{between, char, crlf, digit, many, many1, newline, optional, parser, satisfy,
    sep_end_by, Parser, ParserExt, ParseResult};
use combine::combinator::FnParser;
use combine::primitives::{State, Stream};
use decimal::d128;
use std::fs::File;
use std::io::Read;
use core::commodity::*;
use core::price::*;
use core::symbol::*;
use core::transaction::*;



// HELPERS

/// Takes a tuple of digit characters and converts them to a u32
fn two_digits_to_u32((x, y): (char, char)) -> u32 {
    let x = x.to_digit(10).expect("digit");
    let y = y.to_digit(10).expect("digit");
    (x * 10 + y) as u32
}



// PARSERS

/// Parses at least one whitespace character (space or tab).
fn whitespace<I>(input: State<I>) -> ParseResult<String, I>
where I: Stream<Item=char> {
    many1::<String, _>(satisfy(|c| c == ' ' || c == '\t'))
        .parse_state(input)
}

/// Parses a Unix or Windows style line endings
fn line_ending<I>(input: State<I>) -> ParseResult<String, I>
where I: Stream<Item=char> {
    crlf().map(|x: char| x.to_string())
        .or(newline().map(|x: char| x.to_string()))
        .parse_state(input)
}

/// Wrapped parser for parsing two digits. e.g. 17
fn two_digits<I>() -> FnParser<I, fn (State<I>) -> ParseResult<u32, I>>
where I: Stream<Item=char> {
    fn two_digits_<I>(input: State<I>) -> ParseResult<u32, I>
    where I: Stream<Item=char> {
        (digit(), digit()).map(two_digits_to_u32)
            .parse_state(input)
    }
    parser(two_digits_)
}

/// Parses a date. e.g. 2015-10-17
fn date<I>(input: State<I>) -> ParseResult<Date<Local>, I>
where I: Stream<Item=char> {
    (many::<String, _>(digit()), char('-'), two_digits(), char('-'), two_digits())
        .map(|(year, _, month, _, day)| {
            Local.ymd(year.parse().unwrap(), month, day)
        })
        .parse_state(input)
}

/// Parses an amount
fn amount<I>(input: State<I>) -> ParseResult<d128,I>
where I: Stream<Item=char> {
    (
        optional(char('-')).map(|x| {
            match x {
                Some(_) => "-".to_string(),
                None => "".to_string()
            }
        }),
        many1::<String, _>(satisfy(|c : char| {
            c.is_digit(10) || c == ',' || c == '.'
        }))
    )
        .map(|(sign, numbers)| {
            let mut qty = format!("{}{}", sign, numbers);
            qty = qty.replace(",", "");
            d128::from_str(&qty[..]).unwrap()
        })
        .parse_state(input)
}

/// Parses a quoted symbol
fn quoted_symbol<I>(input: State<I>) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    (char('\"'), many1::<String, _>(satisfy(|c| c != '\"' && c != '\r' && c != '\n')), char('\"'))
        .map(|(_, symbol, _)| Symbol::new(symbol, QuoteOption::Quoted))
        .parse_state(input)
}

/// Parses an unquoted symbol
fn unquoted_symbol<I>(input: State<I>) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    many1::<String, _>(satisfy(|c| "-0123456789; \"\t\r\n".chars().all(|s| s != c)))
        .map(|symbol| Symbol::new(symbol, QuoteOption::Unquoted))
        .parse_state(input)
}

/// Parses a quoted or unquoted symbol
fn symbol<I>(input: State<I>) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    parser(quoted_symbol)
        .or(parser(unquoted_symbol))
        .parse_state(input)
}

/// Parses a commodity in the format of symbol then amount.
fn commodity_symbol_then_amount<I>(input: State<I>) -> ParseResult<Commodity, I>
where I: Stream<Item=char> {
    (parser(symbol), optional(parser(whitespace)), parser(amount))
        .map(|(symbol, opt_whitespace, amount)| {
            let spacing = match opt_whitespace {
                Some(_) => Spacing::Space,
                None => Spacing::NoSpace,
            };
            let render_opts = RenderOptions::new(SymbolPosition::Left, spacing);
            Commodity::new(amount, symbol, render_opts)
        })
        .parse_state(input)
}

/// Parses a commodity in the format of amount then symbol.
fn commodity_amount_then_symbol<I>(input: State<I>) -> ParseResult<Commodity, I>
where I: Stream<Item=char> {
    (parser(amount), optional(parser(whitespace)), parser(symbol))
        .map(|(amount, opt_whitespace, symbol)| {
            let spacing = match opt_whitespace {
                Some(_) => Spacing::Space,
                None => Spacing::NoSpace,
            };
            let render_opts = RenderOptions::new(SymbolPosition::Right, spacing);
            Commodity::new(amount, symbol, render_opts)
        })
        .parse_state(input)
}

/// Parses a commodity
fn commodity<I>(input: State<I>) -> ParseResult<Commodity, I>
where I: Stream<Item=char> {
    parser(commodity_symbol_then_amount)
        .or(parser(commodity_amount_then_symbol))
        .parse_state(input)
}

/// Parses a price entry
fn price<I>(input: State<I>) -> ParseResult<Price, I>
where I: Stream<Item=char> {
    (
        char('P').skip(parser(whitespace)),
        parser(date).skip(parser(whitespace)),
        parser(symbol).skip(parser(whitespace)),
        parser(commodity)
    )
        .map(|(_, date, symbol, commodity)| Price::new(date, symbol, commodity))
        .parse_state(input)
}

/// Parses a price DB file, which contains only price entries.
fn price_db<I>(input: State<I>) -> ParseResult<Vec<Price>, I>
where I: Stream<Item=char> {
    sep_end_by(parser(price), parser(line_ending))
        .parse_state(input)
}

/// Parses transaction status token. e.g. * (cleared) or ! (uncleared)
fn status<I>(input: State<I>) -> ParseResult<Status, I>
where I: Stream<Item=char> {
    char('*').map(|_| Status::Cleared)
        .or(char('!').map(|_| Status::Uncleared))
        .parse_state(input)
}

/// Parses transaction code. e.g. (cheque #802)
fn code<I>(input: State<I>) -> ParseResult<String, I>
where I: Stream<Item=char> {
    between(char('('), char(')'), many(satisfy(|c| c != '\r' && c != '\n' && c != ')')))
        .parse_state(input)
}


// FILES

pub fn parse_pricedb(file_path: &str) -> Vec<Price> {
    let mut file = File::open(file_path).ok().expect("Failed to open file");
    let mut contents = String::new();
    
    file.read_to_string(&mut contents).ok().expect("Failed to read from file");

    let result = parser(price_db).parse(&contents[..]);

    match result {
        Ok((prices, _)) => prices,
        Err(err) => panic!("{}", err),
    }
}



#[cfg(test)]
mod tests {
    use super::{amount, code, commodity, commodity_amount_then_symbol,
        commodity_symbol_then_amount, date, line_ending, price, price_db, quoted_symbol, status,
        symbol, two_digits, two_digits_to_u32, unquoted_symbol, whitespace};
    use chrono::offset::local::Local;
    use chrono::offset::TimeZone;
    use combine::{parser};
    use combine::{Parser};
    use core::commodity::*;
    use core::price::*;
    use core::symbol::*;
    use core::transaction::*;

    // HELPERS

    #[test]
    fn two_digits_to_u32_test() {
        let result = two_digits_to_u32(('2', '7'));
        assert_eq!(result, 27);
    }


    // PARSERS
    
    #[test]
    fn whitespace_empty_is_error()
    {
        let result = parser(whitespace)
            .parse("").map(|x| x.0);
        assert!(result.is_err());
    }

    #[test]
    fn whitespace_space()
    {
        let result = parser(whitespace)
            .parse(" ").map(|x| x.0);
        assert_eq!(result, Ok(" ".to_string()));
    }

    #[test]
    fn whitespace_tab()
    {
        let result = parser(whitespace)
            .parse("\t").map(|x| x.0);
        assert_eq!(result, Ok("\t".to_string()));
    }

    #[test]
    fn line_ending_unix() {
        let result = parser(line_ending)
            .parse("\n").map(|x| x.0);
        assert_eq!(result, Ok("\n".to_string()));
    }

    #[test]
    fn line_ending_windows() {
        let result = parser(line_ending)
            .parse("\r\n").map(|x| x.0);
        assert_eq!(result, Ok("\n".to_string()));
    }

    #[test]
    fn two_digits_test() {
        let result = two_digits()
            .parse("09").map(|x| x.0);
        assert_eq!(result, Ok(9));
    }

    #[test]
    fn date_test() {
        let result = parser(date)
            .parse("2015-10-17").map(|x| x.0);
        assert_eq!(result, Ok(Local.ymd(2015, 10, 17)));
    }

    #[test]
    fn amount_negative_no_fractional_part()
    {
        let result = parser(amount)
            .parse("-1110").map(|x| x.0);
        assert_eq!(result, Ok(d128!(-1110)));
    }

    #[test]
    fn amount_positive_no_fractional_part()
    {
        let result = parser(amount)
            .parse("2,314").map(|x| x.0);
        assert_eq!(result, Ok(d128!(2314)));
    }

    #[test]
    fn amount_negative_with_fractional_part()
    {
        let result = parser(amount)
            .parse("-1,110.38").map(|x| x.0);
        assert_eq!(result, Ok(d128!(-1110.38)));
    }

    #[test]
    fn amount_positive_with_fractional_part()
    {
        let result = parser(amount)
            .parse("24521.793").map(|x| x.0);
        assert_eq!(result, Ok(d128!(24521.793)));
    }

    #[test]
    fn quoted_symbol_test() {
        let result = parser(quoted_symbol)
            .parse("\"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("MUTF2351", QuoteOption::Quoted)));
    }

    #[test]
    fn unquoted_symbol_just_symbol() {
        let result = parser(unquoted_symbol)
            .parse("$").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("$", QuoteOption::Unquoted)));
    }

    #[test]
    fn unquoted_symbol_symbol_and_letters() {
        let result = parser(unquoted_symbol)
            .parse("US$").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("US$", QuoteOption::Unquoted)));
    }

    #[test]
    fn unquoted_symbol_just_letters() {
        let result = parser(unquoted_symbol)
            .parse("AAPL").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("AAPL", QuoteOption::Unquoted)));
    }

    #[test]
    fn symbol_unquoted_test() {
        let result = parser(symbol)
            .parse("$").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("$", QuoteOption::Unquoted)));
    }

    #[test]
    fn symbol_quoted_test() {
        let result = parser(symbol)
            .parse("\"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Symbol::new("MUTF2351", QuoteOption::Quoted)));
    }

    #[test]
    fn commodity_symbol_then_amount_no_whitespace() {
        let result = parser(commodity_symbol_then_amount)
            .parse("$13,245.00").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
    }

    #[test]
    fn commodity_symbol_then_amount_with_whitespace() {
        let result = parser(commodity_symbol_then_amount)
            .parse("$ 13,245.00").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::Space))));
    }

    #[test]
    fn commodity_amount_then_symbol_no_whitespace() {
        let result = parser(commodity_amount_then_symbol)
            .parse("13,245.463AAPL").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.463),
            Symbol::new("AAPL", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::NoSpace))));
    }

    #[test]
    fn commodity_amount_then_symbol_with_whitespace() {
        let result = parser(commodity_amount_then_symbol)
            .parse("13,245.463 \"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space))));
    }

    #[test]
    fn commodity_test_symbol_then_amount() {
        let result = parser(commodity)
            .parse("$13,245.46").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.46),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
    }

    #[test]
    fn commodity_test_amount_then_symbol() {
        let result = parser(commodity)
            .parse("13,245.463 \"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Commodity::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space))));
    }

    #[test]
    fn price_test() {
        let result = parser(price)
            .parse("P 2015-10-25 \"MUTF2351\" $5.42").map(|x| x.0);
        assert_eq!(result, Ok(Price::new(
            Local.ymd(2015, 10, 25),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            Commodity::new(
                d128!(5.42),
                Symbol::new("$", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)))));
    }

    #[test]
    fn price_db_no_records() {
        let result = parser(price_db)
            .parse("").map(|x| x.0);
        assert_eq!(result, Ok(vec![]));
    }

    #[test]
    fn price_db_one_record() {
        let result = parser(price_db)
            .parse("P 2015-10-25 \"MUTF2351\" $5.42").map(|x| x.0);
        assert_eq!(result, Ok(vec![
            Price::new(
                Local.ymd(2015, 10, 25),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Commodity::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)))
        ]));
    }

    #[test]
    fn price_db_multiple_records() {
        let result = parser(price_db)
            .parse("\
                P 2015-10-23 \"MUTF2351\" $5.42\n\
                P 2015-10-25 \"MUTF2351\" $5.98\n\
                P 2015-10-25 AAPL $313.38\n\
            ").map(|x| x.0);
        assert_eq!(result, Ok(vec![
            Price::new(
                Local.ymd(2015, 10, 23),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Commodity::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            Price::new(
                Local.ymd(2015, 10, 25),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Commodity::new(
                    d128!(5.98),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            Price::new(
                Local.ymd(2015, 10, 25),
                Symbol::new("AAPL", QuoteOption::Unquoted),
                Commodity::new(
                    d128!(313.38),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)))
        ]));
    }

    #[test]
    fn status_cleared() {
        let result = parser(status)
            .parse("*").map(|x| x.0);
        assert_eq!(result, Ok(Status::Cleared));
    }

    #[test]
    fn status_uncleared() {
        let result = parser(status)
            .parse("!").map(|x| x.0);
        assert_eq!(result, Ok(Status::Uncleared));
    }

    #[test]
    fn code_empty() {
        let result = parser(code)
            .parse("()").map(|x| x.0);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn code_short() {
        let result = parser(code)
            .parse("(89)").map(|x| x.0);
        assert_eq!(result, Ok("89".to_string()));
    }

    #[test]
    fn code_long() {
        let result = parser(code)
            .parse("(conf# abc-123-DEF)").map(|x| x.0);
        assert_eq!(result, Ok("conf# abc-123-DEF".to_string()));
    }

}