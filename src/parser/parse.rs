use rust_core::str::FromStr;
use chrono::Date;
use chrono::offset::Local;
use chrono::offset::TimeZone;
use combine::{between, many, many1, optional, parser, satisfy, sep_by1, sep_end_by, skip_many, try,
    Parser, ParseResult};
use combine::char::{alpha_num, char, crlf, digit, newline};
use combine::combinator::FnParser;
use combine::primitives::{Stream};
use decimal::d128;
use std::fs::File;
use std::io::Read;
use core::amount::*;
use core::price::*;
use core::symbol::*;
use core::transaction::*;
use parser::ast::*;



// HELPERS

/// Takes a tuple of digit characters and converts them to a u32
fn two_digits_to_u32((x, y): (char, char)) -> u32 {
    let x = x.to_digit(10).expect("digit");
    let y = y.to_digit(10).expect("digit");
    (x * 10 + y) as u32
}



// PARSERS

/// Parses at least one whitespace character (space or tab).
fn whitespace<I>(input: I) -> ParseResult<String, I>
where I: Stream<Item=char> {
    many1::<String, _>(satisfy(|c| c == ' ' || c == '\t'))
        .parse_stream(input)
}

/// Parses a Unix or Windows style line endings
fn line_ending<I>(input: I) -> ParseResult<String, I>
where I: Stream<Item=char> {
    crlf().map(|x: char| x.to_string())
        .or(newline().map(|x: char| x.to_string()))
        .parse_stream(input)
}

/// Wrapped parser for parsing two digits. e.g. 17
fn two_digits<I>() -> FnParser<I, fn (I) -> ParseResult<u32, I>>
where I: Stream<Item=char> {
    fn two_digits_<I>(input: I) -> ParseResult<u32, I>
    where I: Stream<Item=char> {
        (digit(), digit()).map(two_digits_to_u32)
            .parse_stream(input)
    }
    parser(two_digits_)
}

/// Parses a date. e.g. 2015-10-17
fn date<I>(input: I) -> ParseResult<Date<Local>, I>
where I: Stream<Item=char> {
    (many::<String, _>(digit()), char('-'), two_digits(), char('-'), two_digits())
        .map(|(year, _, month, _, day)| {
            Local.ymd(year.parse().unwrap(), month, day)
        })
        .parse_stream(input)
}

/// Parses a quantity
fn quantity<I>(input: I) -> ParseResult<d128,I>
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
        .parse_stream(input)
}

/// Parses a quoted symbol
fn quoted_symbol<I>(input: I) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    (char('\"'), many1::<String, _>(satisfy(|c| c != '\"' && c != '\r' && c != '\n')), char('\"'))
        .map(|(_, symbol, _)| Symbol::new(symbol, QuoteOption::Quoted))
        .parse_stream(input)
}

/// Parses an unquoted symbol
fn unquoted_symbol<I>(input: I) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    many1::<String, _>(satisfy(|c| "-0123456789; \"\t\r\n".chars().all(|s| s != c)))
        .map(|symbol| Symbol::new(symbol, QuoteOption::Unquoted))
        .parse_stream(input)
}

/// Parses a quoted or unquoted symbol
fn symbol<I>(input: I) -> ParseResult<Symbol, I>
where I: Stream<Item=char> {
    parser(quoted_symbol)
        .or(parser(unquoted_symbol))
        .parse_stream(input)
}

/// Parses an amount in the format of symbol then quantity.
fn amount_symbol_then_quantity<I>(input: I) -> ParseResult<Amount, I>
where I: Stream<Item=char> {
    (parser(symbol), optional(parser(whitespace)), parser(quantity))
        .map(|(symbol, opt_whitespace, quantity)| {
            let spacing = match opt_whitespace {
                Some(_) => Spacing::Space,
                None => Spacing::NoSpace,
            };
            let render_opts = RenderOptions::new(SymbolPosition::Left, spacing);
            Amount::new(quantity, symbol, render_opts)
        })
        .parse_stream(input)
}

/// Parses an amount in the format of quantity then symbol.
fn amount_quantity_then_symbol<I>(input: I) -> ParseResult<Amount, I>
where I: Stream<Item=char> {
    (parser(quantity), optional(parser(whitespace)), parser(symbol))
        .map(|(quantity, opt_whitespace, symbol)| {
            let spacing = match opt_whitespace {
                Some(_) => Spacing::Space,
                None => Spacing::NoSpace,
            };
            let render_opts = RenderOptions::new(SymbolPosition::Right, spacing);
            Amount::new(quantity, symbol, render_opts)
        })
        .parse_stream(input)
}

/// Parses an amount
fn amount<I>(input: I) -> ParseResult<Amount, I>
where I: Stream<Item=char> {
    parser(amount_symbol_then_quantity)
        .or(parser(amount_quantity_then_symbol))
        .parse_stream(input)
}

/// Parses an amount or an inferred amount
fn amount_or_inferred<I>(input: I) -> ParseResult<(AmountSource, Option<Amount>), I>
where I: Stream<Item=char> {
    optional(parser(amount))
        .map(|opt_amount| {
            let source = match opt_amount {
                Some(_) => AmountSource::Provided,
                None => AmountSource::Inferred
            };
            (source, opt_amount)
        })
        .parse_stream(input)
}

/// Parses a price entry
fn price<I>(input: I) -> ParseResult<Price, I>
where I: Stream<Item=char> {
    (
        char('P').skip(parser(whitespace)),
        parser(date).skip(parser(whitespace)),
        parser(symbol).skip(parser(whitespace)),
        parser(amount)
    )
        .map(|(_, date, symbol, amount)| Price::new(date, symbol, amount))
        .parse_stream(input)
}

/// Parses a price DB file, which contains only price entries.
fn price_db<I>(input: I) -> ParseResult<Vec<Price>, I>
where I: Stream<Item=char> {
    sep_end_by(parser(price), parser(line_ending))
        .parse_stream(input)
}

/// Parses transaction status token. e.g. * (cleared) or ! (uncleared)
fn status<I>(input: I) -> ParseResult<Status, I>
where I: Stream<Item=char> {
    char('*').map(|_| Status::Cleared)
        .or(char('!').map(|_| Status::Uncleared))
        .parse_stream(input)
}

/// Parses transaction code. e.g. (cheque #802)
fn code<I>(input: I) -> ParseResult<String, I>
where I: Stream<Item=char> {
    between(char('('), char(')'), many(satisfy(|c| c != '\r' && c != '\n' && c != ')')))
        .parse_stream(input)
}

/// Parses a payee.
fn payee<I>(input: I) -> ParseResult<String,I>
where I: Stream<Item=char> {
    many1(satisfy(|c| c != ';' && c != '\n' && c != '\r'))
        .parse_stream(input)
}

/// Parses a comment.
fn comment<I>(input: I) -> ParseResult<String,I>
where I: Stream<Item=char> {
    char(';').with(many(satisfy(|c| c != '\r' && c != '\n')))
        .parse_stream(input)
}

// Parses a comment line, which may start with whitespace.
fn comment_line<I>(input: I) -> ParseResult<String,I>
where I: Stream<Item=char> {
    many::<String, _>(parser(whitespace)).with(parser(comment)).skip(parser(line_ending))
        .parse_stream(input)
}

/// Parses a transaction header.
fn header<I>(input: I) -> ParseResult<Header,I>
where I: Stream<Item=char> {
    (
        parser(date).skip(parser(whitespace)),
        parser(status).skip(parser(whitespace)),
        optional(parser(code).skip(parser(whitespace))),
        parser(payee),
        optional(parser(comment))
    )
        .map(|(date, status, code, payee, comment)|
            Header::new(date, status, code, payee, comment))
        .parse_stream(input)
}

/// Parses a sub-account name, which must be alphanumeric.
fn sub_account<I>(input: I) -> ParseResult<String,I>
where I: Stream<Item=char> {
    many1(alpha_num())
        .parse_stream(input)
}

/// Parses an account, made up of sub-accounts separated by colons.
fn account<I>(input: I) -> ParseResult<Vec<String>,I>
where I: Stream<Item=char> {
    sep_by1(parser(sub_account), char(':'))
        .parse_stream(input)
}

/// Parses a transaction posting.
fn posting<I>(input: I) -> ParseResult<RawPosting, I>
where I: Stream<Item=char> {
    (
        parser(account).skip(optional(parser(whitespace))),
        parser(amount_or_inferred).skip(optional(parser(whitespace))),
        optional(parser(comment))
    )
        .map(|(sub_accounts, (amount_source, opt_amount), opt_comment)|
            RawPosting::new(sub_accounts, opt_amount, amount_source, opt_comment))
        .parse_stream(input)
}

/// Parses a transaction posting line, which must begin with whitespace.
fn posting_line<I>(input: I) -> ParseResult<RawPosting, I>
where I: Stream<Item=char> {
    many1::<String, _>(parser(whitespace)).with(parser(posting)).skip(parser(line_ending))
        .parse_stream(input)
}

/// Parses a whole transaction.
fn transaction<I>(input: I) -> ParseResult<ParseTree, I>
where I: Stream<Item=char> {
    (
        parser(header).skip(parser(line_ending)),
        many1(try(parser(comment_line).map(|_| None))
                .or(try(parser(posting_line).map(|p| Some(p)))))
    )
        .map(|(header, postings) : (Header, Vec<Option<RawPosting>>)| {
            let raw_postings = postings.into_iter().filter_map(|p| p).collect();
            ParseTree::Transaction(header, raw_postings)
        })
        .parse_stream(input)
}

/// Parses and discards any number of comment or empty line.
fn skip_comment_or_empty_lines<I>(input: I) -> ParseResult<(), I>
where I: Stream<Item=char> {
    skip_many(skip_many(parser(whitespace))
            .skip(optional(parser(comment)))
            .skip(parser(line_ending)))
        .parse_stream(input)
}

/// Parses a complete ledger, extracting transactions and prices.
fn ledger<I>(input: I) -> ParseResult<Vec<ParseTree>, I>
where I: Stream<Item=char> {
    // skip one or more comment or empty lines
    // parse transactions or prices separated, which may be separated bycomment or empty lines
    parser(skip_comment_or_empty_lines)
        .with(many(
            parser(transaction)
                .or(parser(price).map(|p| ParseTree::Price(p)))
                .skip(parser(skip_comment_or_empty_lines))))
        .parse_stream(input)
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

pub fn parse_ledger(file_path: &str) -> Vec<ParseTree> {
    let mut file = File::open(file_path).ok().expect("Failed to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents).ok().expect("Failed to read from file");

    let result = parser(ledger).parse(&contents[..]);

    // TODO: Should return result value rather than panic here
    match result {
        Ok((tree, _)) => tree,
        Err(err) => panic!("{}", err),
    }
}



#[cfg(test)]
mod tests {
    use super::{account, amount, amount_quantity_then_symbol, amount_or_inferred,
        amount_symbol_then_quantity, code, comment, comment_line, skip_comment_or_empty_lines,
        date, header, ledger, line_ending, payee, posting, posting_line, price, price_db, quantity,
        quoted_symbol, status, sub_account, symbol, transaction, two_digits, two_digits_to_u32,
        unquoted_symbol, whitespace};
    use chrono::offset::Local;
    use chrono::offset::TimeZone;
    use combine::{parser};
    use combine::{Parser};
    use core::amount::*;
    use core::price::*;
    use core::symbol::*;
    use core::transaction::*;
    use parser::ast::*;

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
    fn quantity_negative_no_fractional_part()
    {
        let result = parser(quantity)
            .parse("-1110").map(|x| x.0);
        assert_eq!(result, Ok(d128!(-1110)));
    }

    #[test]
    fn quantity_positive_no_fractional_part()
    {
        let result = parser(quantity)
            .parse("2,314").map(|x| x.0);
        assert_eq!(result, Ok(d128!(2314)));
    }

    #[test]
    fn quantity_negative_with_fractional_part()
    {
        let result = parser(quantity)
            .parse("-1,110.38").map(|x| x.0);
        assert_eq!(result, Ok(d128!(-1110.38)));
    }

    #[test]
    fn quantity_positive_with_fractional_part()
    {
        let result = parser(quantity)
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
    fn amount_symbol_then_quantity_no_whitespace() {
        let result = parser(amount_symbol_then_quantity)
            .parse("$13,245.00").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
    }

    #[test]
    fn amount_symbol_then_quantity_with_whitespace() {
        let result = parser(amount_symbol_then_quantity)
            .parse("$ 13,245.00").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.00),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::Space))));
    }

    #[test]
    fn amount_quantity_then_symbol_no_whitespace() {
        let result = parser(amount_quantity_then_symbol)
            .parse("13,245.463AAPL").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("AAPL", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::NoSpace))));
    }

    #[test]
    fn amount_quantity_then_symbol_with_whitespace() {
        let result = parser(amount_quantity_then_symbol)
            .parse("13,245.463 \"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space))));
    }

    #[test]
    fn amount_test_symbol_then_quantity() {
        let result = parser(amount)
            .parse("$13,245.46").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.46),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))));
    }

    #[test]
    fn amount_test_quantity_then_symbol() {
        let result = parser(amount)
            .parse("13,245.463 \"MUTF2351\"").map(|x| x.0);
        assert_eq!(result, Ok(Amount::new(
            d128!(13245.463),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            RenderOptions::new(SymbolPosition::Right, Spacing::Space))));
    }

    #[test]
    fn amount_or_inferred_amount_provided() {
        let result = parser(amount_or_inferred)
            .parse("$13,245.46").map(|x| x.0);
        assert_eq!(result, Ok((AmountSource::Provided, Some(Amount::new(
            d128!(13245.46),
            Symbol::new("$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))))));
    }

    #[test]
    fn amount_or_inferred_no_amount() {
        let result = parser(amount_or_inferred)
            .parse("").map(|x| x.0);
        assert_eq!(result, Ok((AmountSource::Inferred, None)));
    }

    #[test]
    fn price_test() {
        let result = parser(price)
            .parse("P 2015-10-25 \"MUTF2351\" $5.42").map(|x| x.0);
        assert_eq!(result, Ok(Price::new(
            Local.ymd(2015, 10, 25),
            Symbol::new("MUTF2351", QuoteOption::Quoted),
            Amount::new(
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
                Amount::new(
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
                Amount::new(
                    d128!(5.42),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            Price::new(
                Local.ymd(2015, 10, 25),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                Amount::new(
                    d128!(5.98),
                    Symbol::new("$", QuoteOption::Unquoted),
                    RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            Price::new(
                Local.ymd(2015, 10, 25),
                Symbol::new("AAPL", QuoteOption::Unquoted),
                Amount::new(
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

    #[test]
    fn payee_empty_payee_is_error() {
        let result = parser(payee)
            .parse("").map(|x| x.0);
        assert!(result.is_err());
    }

    #[test]
    fn payee_single_character() {
        let result = parser(payee)
            .parse("Z").map(|x| x.0);
        assert_eq!(result, Ok("Z".to_string()));
    }

    #[test]
    fn payee_short() {
        let result = parser(payee)
            .parse("WonderMart").map(|x| x.0);
        assert_eq!(result, Ok("WonderMart".to_string()));
    }

    #[test]
    fn payee_long() {
        let result = parser(payee)
            .parse("WonderMart - groceries, kitchen supplies (pot), light bulbs").map(|x| x.0);
        assert_eq!(result,
            Ok("WonderMart - groceries, kitchen supplies (pot), light bulbs".to_string()));
    }
    
    #[test]
    fn comment_empty() {
        let result = parser(comment)
            .parse(";").map(|x| x.0);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn comment_no_leading_space() {
        let result = parser(comment)
            .parse(";Comment").map(|x| x.0);
        assert_eq!(result, Ok("Comment".to_string()));
    }

    #[test]
    fn comment_with_leading_space() {
        let result = parser(comment)
            .parse("; Comment").map(|x| x.0);
        assert_eq!(result, Ok(" Comment".to_string()));
    }

    #[test]
    fn comment_line_with_leading_whitespace() {
        let result = parser(comment_line)
            .parse("  ;Comment\r\n").map(|x| x.0);
        assert_eq!(result, Ok("Comment".to_string()));
    }

    #[test]
    fn comment_line_no_leading_whitespace() {
        let result = parser(comment_line)
            .parse(";Comment\r\n").map(|x| x.0);
        assert_eq!(result, Ok("Comment".to_string()));
    }

    #[test]
    fn header_full() {
        let result = parser(header)
            .parse("2015-10-20 * (conf# abc-123) Payee ;Comment").map(|x| x.0);
        assert_eq!(result, Ok(Header::new(
            Local.ymd(2015, 10, 20),
            Status::Cleared,
            Some("conf# abc-123".to_string()),
            "Payee ".to_string(),
            Some("Comment".to_string()))));
    }

    #[test]
    fn header_with_code_and_no_comment() {
        let result = parser(header)
            .parse("2015-10-20 ! (conf# abc-123) Payee").map(|x| x.0);
        assert_eq!(result, Ok(Header::new(
            Local.ymd(2015, 10, 20),
            Status::Uncleared,
            Some("conf# abc-123".to_string()),
            "Payee".to_string(),
            None)));
    }

    #[test]
    fn header_with_comment_and_no_code() {
        let result = parser(header)
            .parse("2015-10-20 * Payee ;Comment").map(|x| x.0);
        assert_eq!(result, Ok(Header::new(
            Local.ymd(2015, 10, 20),
            Status::Cleared,
            None,
            "Payee ".to_string(),
            Some("Comment".to_string()))));
    }

    #[test]
    fn header_with_no_code_or_comment() {
        let result = parser(header)
            .parse("2015-10-20 * Payee").map(|x| x.0);
        assert_eq!(result, Ok(Header::new(
            Local.ymd(2015, 10, 20),
            Status::Cleared,
            None,
            "Payee".to_string(),
            None)));
    }

    #[test]
    fn sub_account_alphanumeric() {
        let result = parser(sub_account)
            .parse("AZaz09").map(|x| x.0);
        assert_eq!(result, Ok("AZaz09".to_string()));
    }

    #[test]
    fn sub_account_can_start_with_digits() {
        let result = parser(sub_account)
            .parse("123abcABC").map(|x| x.0);
        assert_eq!(result, Ok("123abcABC".to_string()));
    }

    #[test]
    fn account_single_level() {
        let result = parser(account)
            .parse("Expenses").map(|x| x.0);
        assert_eq!(result, Ok(vec!["Expenses".to_string()]));
    }

    #[test]
    fn account_multiple_level() {
        let result = parser(account)
            .parse("Expenses:Food:Groceries").map(|x| x.0);
        assert_eq!(result, Ok(vec![
            "Expenses".to_string(),
            "Food".to_string(),
            "Groceries".to_string()
        ]));
    }

    #[test]
    fn posting_with_all_components() {
        let result = parser(posting)
            .parse("Assets:Savings\t$45.00\t;comment").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            Some(Amount::new(
                d128!(45.00),
                Symbol::new("$".to_string(), QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            AmountSource::Provided,
            Some("comment".to_string()))));
    }

    #[test]
    fn posting_with_all_components_alternate_amount() {
        let result = parser(posting)
            .parse("Assets:Investments\t13.508 \"MUTF2351\"\t;comment").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Investments".to_string()
            ],
            Some(Amount::new(
                d128!(13.508),
                Symbol::new("MUTF2351".to_string(), QuoteOption::Quoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::Space))),
            AmountSource::Provided,
            Some("comment".to_string()))));
    }

    #[test]
    fn posting_with_amount_no_comment() {
        let result = parser(posting)
            .parse("Assets:Savings\t$45.00").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            Some(Amount::new(
                d128!(45.00),
                Symbol::new("$".to_string(), QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
            AmountSource::Provided,
            None)));
    }

    #[test]
    fn posting_inferred_amount_with_comment() {
        let result = parser(posting)
            .parse("Assets:Savings\t;comment").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            None,
            AmountSource::Inferred,
            Some("comment".to_string()))));
    }

    #[test]
    fn posting_inferred_amount_no_comment() {
        let result = parser(posting)
            .parse("Assets:Savings").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            None,
            AmountSource::Inferred,
            None)));
    }

    #[test]
    fn posting_line_begins_with_spaces() {
        let result = parser(posting_line)
            .parse("  Assets:Savings\r\n").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            None,
            AmountSource::Inferred,
            None)));
    }

    #[test]
    fn posting_line_begins_with_tab() {
        let result = parser(posting_line)
            .parse("\tAssets:Savings\r\n").map(|x| x.0);
        assert_eq!(result, Ok(RawPosting::new(
            vec![
                "Assets".to_string(),
                "Savings".to_string()
            ],
            None,
            AmountSource::Inferred,
            None)));
    }

    #[test]
    fn transaction_basic() {
        let result = parser(transaction)
            .parse("\
                2016-06-07 * Basic transaction ;comment\n\
                \tExpenses:Groceries    $45.00\n\
                \tLiabilities:Credit\n\
            ").map(|x| x.0);
        assert_eq!(result, Ok(ParseTree::Transaction(
            Header::new(
                Local.ymd(2016, 6, 7),
                Status::Cleared,
                None,
                "Basic transaction ".to_string(),
                Some("comment".to_string())),
            vec![
                RawPosting::new(
                    vec![
                        "Expenses".to_string(),
                        "Groceries".to_string(),
                    ],
                    Some(Amount::new(
                        d128!(45.00),
                        Symbol::new("$".to_string(), QuoteOption::Unquoted),
                        RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
                    AmountSource::Provided,
                    None),
                RawPosting::new(
                    vec![
                        "Liabilities".to_string(),
                        "Credit".to_string(),
                    ],
                    None,
                    AmountSource::Inferred,
                    None)
            ]
        )));
    }

    #[test]
    fn transaction_with_comment() {
        let result = parser(transaction)
            .parse("\
                2016-06-07 * Basic transaction ;comment\n\
                ; a comment in a transaction
                \tExpenses:Groceries    $45.00\n\
                \tLiabilities:Credit\n\
            ").map(|x| x.0);
        assert_eq!(result, Ok(ParseTree::Transaction(
            Header::new(
                Local.ymd(2016, 6, 7),
                Status::Cleared,
                None,
                "Basic transaction ".to_string(),
                Some("comment".to_string())),
            vec![
                RawPosting::new(
                    vec![
                        "Expenses".to_string(),
                        "Groceries".to_string(),
                    ],
                    Some(Amount::new(
                        d128!(45.00),
                        Symbol::new("$".to_string(), QuoteOption::Unquoted),
                        RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace))),
                    AmountSource::Provided,
                    None),
                RawPosting::new(
                    vec![
                        "Liabilities".to_string(),
                        "Credit".to_string(),
                    ],
                    None,
                    AmountSource::Inferred,
                    None)
            ]
        )));
    }

    #[test]
    fn skip_comment_or_empty_lines_comment() {
        let result = parser(skip_comment_or_empty_lines)
            .parse("; comment\r\n").map(|x| x.0);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn skip_comment_or_empty_lines_comments() {
        let result = parser(skip_comment_or_empty_lines)
            .parse("; comment\r\n; another comment\r\n").map(|x| x.0);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn skip_comment_or_empty_lines_empty_line() {
        let result = parser(skip_comment_or_empty_lines)
            .parse("  \r\n").map(|x| x.0);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn skip_comment_or_empty_lines_empty_lines() {
        let result = parser(skip_comment_or_empty_lines)
            .parse("  \r\n\r\n\t\r\n").map(|x| x.0);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn skip_comment_or_empty_lines_mix() {
        let result = parser(skip_comment_or_empty_lines)
            .parse("  \r\n; comment\r\n\t; indented comment\r\n\r\n").map(|x| x.0);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn ledger_single_transaction() {
        let result = parser(ledger)
            .parse("; Preamble\n\
                \n\
                2016-06-07 * Basic transaction ;comment\n\
                \tExpenses:Groceries    $45.00\n\
                \tLiabilities:Credit\n\
                \n\
            ").map(|x| x.0);
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn ledger_small_sample() {
        let result = parser(ledger)
            .parse("; Preamble\n\
                \n\
                2016-06-07 * Basic transaction ;comment\n\
                \tExpenses:Groceries    $45.00\n\
                \tLiabilities:Credit\n\
                \t\n\
                P 2016-06-07 \"MUTF2351\" $4.56\n\
                P 2016-06-07 AAPL $23.33\n\
                \n\
                ; Separator\n\
                \n\
                2016-06-07 * Basic transaction ;comment\n\
                \tExpenses:Groceries    $45.00\n\
                \tLiabilities:Credit\n\
            ").map(|x| x.0);
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().len(), 4);
    }

}