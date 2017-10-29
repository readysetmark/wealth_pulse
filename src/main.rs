extern crate wealth_pulse;

use wealth_pulse::parser::parse::{parse_ledger, parse_pricedb};
use std::env;

// MAIN

fn main() {
    let pricedb_filepath = env::var("WEALTH_PULSE_PRICES_FILE")
        .expect("Could not read WEALTH_PULSE_PRICES_FILE environment variable");
    let ledger_filepath = env::var("LEDGER_FILE")
        .expect("Could not read LEDGER_FILE environment variable");

    let prices = parse_pricedb(&pricedb_filepath);
    println!("Parsed pricedb file: {}", pricedb_filepath);
    println!("  {} prices", prices.len());

    let (num_txs, postings, prices) = parse_ledger(&ledger_filepath);
    println!("Parsed ledger file: {}", ledger_filepath);
    println!("  {} transactions", num_txs);
    println!("  {} postings", postings.len());
    println!("  {} prices", prices.len());

    // for price in &prices {
    //     println!("{}", price);
    // }

}
