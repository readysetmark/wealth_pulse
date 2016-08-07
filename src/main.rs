extern crate wealth_pulse;

//use wealth_pulse::parser::chomp::{parse_pricedb};
use wealth_pulse::parser::combine::{parse_pricedb, parse_ledger};

// MAIN

fn main() {
    let pricedb_filepath = "/Users/mark/Nexus/Documents/finances/ledger/.pricedb";
    let ledger_filepath = "/Users/mark/Nexus/Documents/finances/ledger/ledger.dat";

    let prices = parse_pricedb(pricedb_filepath);
    println!("Parsed {} prices", prices.len());

    let tree = parse_ledger(ledger_filepath);
    println!("Parsed {} tree items", tree.len());

    // for price in &prices {
    //     println!("{}", price);
    // }

}
