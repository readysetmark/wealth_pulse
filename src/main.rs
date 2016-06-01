extern crate wealth_pulse;

//use wealth_pulse::parser::chomp::{pricedb_file};
use wealth_pulse::parser::combine::{parse_pricedb};

// MAIN

fn main() {
    let pricedb_filepath = "/Users/mark/Nexus/Documents/finances/ledger/.pricedb";

    let prices = parse_pricedb(pricedb_filepath);

    // for price in &prices {
    //     println!("{}", price);
    // }

    println!("Parsed {} prices", prices.len());
}
