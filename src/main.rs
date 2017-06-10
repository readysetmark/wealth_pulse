extern crate wealth_pulse;

//use wealth_pulse::parser::chomp::{parse_pricedb};
use wealth_pulse::parser::combine::{parse_pricedb, parse_ledger};
use std::env;

// MAIN

fn main() {
    let pricedb_filepath = env::var("WEALTH_PULSE_PRICES_FILE")
        .expect("Could not read WEALTH_PULSE_PRICES_FILE environment variable");
    let ledger_filepath = env::var("LEDGER_FILE")
        .expect("Could not read LEDGER_FILE environment variable");

    let prices = parse_pricedb(&pricedb_filepath);
    println!("Parsed {} prices", prices.len());

    let tree = parse_ledger(&ledger_filepath);
    println!("Parsed {} tree items", tree.len());

    // for price in &prices {
    //     println!("{}", price);
    // }

}
