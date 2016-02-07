#[macro_use]
extern crate chomp;

use std::fs::File;
use std::io::Read;
use std::str;
use chomp::{Input, U8Result, parse_only};
use chomp::{count, eof, option, or, sep_by, string, take_while, take_while1, token};
use chomp::ascii::{digit};
use chomp::buffer::{Source, Stream, StreamError};

fn to_i32(slice: Vec<u8>) -> i32 {
    slice.iter().fold(0, |acc, &d| (acc * 10) + ((d - ('0' as u8)) as i32))
}

fn year(i: Input<u8>) -> U8Result<i32> {
    count(i, 4, |i| digit(i)).map(to_i32)
}

fn month(i: Input<u8>) -> U8Result<i32> {
    count(i, 2, |i| digit(i)).map(to_i32)
}

fn day(i: Input<u8>) -> U8Result<i32> {
    count(i, 2, |i| digit(i)).map(to_i32)
}

fn date(i: Input<u8>) -> U8Result<(i32, i32, i32)> {
    parse!{i;
        let year =  year();
                    token(b'-');
        let month = month();
                    token(b'-');
        let day =   day();
        ret (year, month, day)
    }
}

fn is_whitespace(c: u8) -> bool {
    c == b'\t' || c == b' '
}

fn whitespace(i: Input<u8>) -> U8Result<bool> {
    take_while(i, is_whitespace).map(|ws| ws.len() > 0)
}

fn mandatory_whitespace(i: Input<u8>) -> U8Result<()> {
    take_while1(i, is_whitespace).map(|_| ())
}


fn is_quoted_symbol_char(c: u8) -> bool {
    c != b'\"' && c != b'\r' && c != b'\n'
}

fn quoted_symbol(i: Input<u8>) -> U8Result<&str> {
    parse!{i;
        token(b'\"');
        let symbol = take_while1(is_quoted_symbol_char);
        token(b'\"');
        ret str::from_utf8(symbol).unwrap()
    }
}

fn is_digit(c: u8) -> bool {
    b'0' <= c && c <= b'9'
}

fn is_newline(c: u8) -> bool {
    c == b'\r' && c == b'\n'
}

fn is_unquoted_symbol_char(c: u8) -> bool {
    //"-0123456789; \"\t\r\n".as_bytes().iter().all(|b| b != c))
    c != b'-' && c != b';' && c != b'\"' && !is_newline(c)
     && !is_digit(c) && !is_whitespace(c)
}

fn unquoted_symbol(i: Input<u8>) -> U8Result<&str> {
    take_while1(i, is_unquoted_symbol_char).map(|b| str::from_utf8(b).unwrap())
}

fn symbol(i: Input<u8>) -> U8Result<&str> {
    or(i, quoted_symbol, unquoted_symbol)
}


fn is_quantity_char(c: u8) -> bool {
    is_digit(c) || c == b'.' || c == b','
}

fn make_quantity(sign: u8, number: &[u8]) -> String {
    let mut qty = String::new();
    if sign == b'-' {
        qty.push_str(str::from_utf8(&[sign]).unwrap());
    }
    qty.push_str(str::from_utf8(number).unwrap());
    qty.replace(",", "");
    qty
}

fn quantity(i: Input<u8>) -> U8Result<String> {
    parse!{i;
        let sign = option(|i| token(i, b'-'), b'+');
        let number = take_while1(is_quantity_char);      
        ret make_quantity(sign, number)
    }
}

fn amount_symbol_then_quantity(i: Input<u8>) -> U8Result<(String, &str)> {
    parse!{i;
        let symbol = symbol();
        whitespace();
        let quantity = quantity();
        ret (quantity, symbol)
    }
}

fn amount_quantity_then_symbol(i: Input<u8>) -> U8Result<(String, &str)> {
    parse!{i;
        let quantity = quantity();
        whitespace();
        let symbol = symbol();
        ret (quantity, symbol)
    }
}

fn amount(i: Input<u8>) -> U8Result<(String, &str)> {
    or(i, amount_symbol_then_quantity, amount_quantity_then_symbol)
}



fn price(i: Input<u8>) -> U8Result<((i32, i32, i32), &str, (String, &str))> {
    parse!{i;
        token(b'P');
        mandatory_whitespace();
        let date = date();
        mandatory_whitespace();
        let symbol = symbol();
        mandatory_whitespace();
        let amount = amount();
        ret (date, symbol, amount)
    }
}


fn line_ending(i: Input<u8>) -> U8Result<()> {
    or(i,
        |i| token(i, b'\n').map(|_| ()),
        |i| string(i, b"\r\n").map(|_| ()))
}


fn price_line(i: Input<u8>) -> U8Result<((i32, i32, i32), &str, (String, &str))> {
    parse!{i;
        let price = price();
        line_ending();
        ret price
    }
}

// fn price_db(i: Input<u8>) -> U8Result<Vec<((i32, i32, i32), &str, (String, &str))>> {
//     parse!{i;
//         let prices = sep_by(price, line_ending);
//         eof();
//         ret prices
//     }
// }


fn main() {
    // println!("{:?}", parse_only(mandatory_whitespace, b" "));
    // println!("{:?}", parse_only(date, b"2016-02-06"));
    // println!("{:?}", parse_only(symbol, b"\"AIM1651\""));
    // println!("{:?}", parse_only(symbol, b"$"));
    // println!("{:?}", parse_only(amount, b"$-5.82"));
    // println!("{:?}", parse_only(price, b"P 2016-02-06 \"AIM1651\" $5.82"));
    // println!("{:?}", parse_only(price_db, b"P 2016-02-06 \"AIM1651\" $5.82\r\nP 2016-02-07 \"AIM1651\" $5.85"));

    let price_db_filepath = "/Users/mark/Nexus/Documents/finances/ledger/.pricedb";
    let file = File::open(price_db_filepath).ok().expect("Failed to open file");

    let mut source = Source::new(file);
    let mut n = 0;

    loop {
        match source.parse(price_line) {
            Ok(_)                        => n += 1,
            Err(StreamError::Retry)      => {}, // Needed to refill buffer when necessary
            Err(StreamError::EndOfInput) => break,
            Err(e)                       => { panic!("{:?}", e); }
        }
    }

    println!("Parsed {} prices", n);

    // match source.parse(price_db) {
    //     Ok(prices)  => println!("Parsed {} prices", prices.len()),
    //     Err(e)      => println!("Uhm, wat? {:?}", e)
    // }
}
