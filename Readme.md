Wealth Pulse
============

[![Build Status](https://travis-ci.org/readysetmark/wealth_pulse.svg?branch=master)](https://travis-ci.org/readysetmark/wealth_pulse)

Wealth Pulse personal finance tracking application, of sorts, supporting 
double-entry accounting. "Of sorts" because it is only used for reporting --
data entry is done via any text editor in a ledger file. Wealth Pulse provides
web-based reporting, allowing for rich tables and charts.

Wealth Pulse is re-make of [Ledger][ledger], which only provides command line
reporting, and takes inspiration from [Penny][penny].


How to Run
----------

Run via Cargo:

```
> cargo run --release
```

Run tests via Cargo:

```
> cargo test
```


Terminology
-----------

Temporary section until I build this stuff out:
    Transaction Header (Transaction)
    Posting (Transaction entry line)
    Commodity (has an amount and symbol)


Project Plan
------------

Ledger
* [ ] Parse ledger file
* [ ] Validate and transform

Configuration
* [ ] Parse configuration

PriceDB
* [x] Parse pricedb file
* [ ] Serialize pricedb file
* [ ] Fetch new prices and store

Reports
* [ ] Balance report
* [ ] Register report
* [ ] Net worth report


[ledger]: http://www.ledger-cli.org/
[penny]: http://massysett.github.io/penny/