Wealth Pulse
============

[![Build Status](https://travis-ci.org/readysetmark/wealth_pulse.svg?branch=master)](https://travis-ci.org/readysetmark/wealth_pulse)
[![Build status](https://ci.appveyor.com/api/projects/status/63mehh2jefaslhj0/branch/master?svg=true)](https://ci.appveyor.com/project/readysetmark/wealth-pulse/branch/master)

Wealth Pulse is a personal finance tracking application, of sorts, supporting double-entry
accounting. "Of sorts" because it is only used for reporting -- data entry is done via any text
editor in a ledger file. Wealth Pulse provides web-based reporting, allowing for rich tables and
charts.

Wealth Pulse is re-make of [Ledger][ledger] and takes inspiration from [Penny][penny].


How to Run
----------

Since this project is still in its infancy, there are no pre-compiled binaries yet. You'll need the
code to run anything.

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

* Transaction Header (Transaction)
* Posting (Transaction entry line)
* Commodity (has an amount and symbol)


Tasks
-----

### Ledger Loading

* [x] Parse ledger file
* [ ] Validate and transform:
    * [ ] Autobalance transactions
    * [ ] Ensure all transactions balance
    * [ ] Transform transactions into list of postings
* [ ] Collect ledger stats:
    * [ ] Last modified date/time
    * [ ] Number of transactions
    * [ ] Number of postings
    * [ ] Number of price entries
* [ ] Handle parsing/validation errors gracefully

### Configuration Loading

* [ ] Parse configuration

### PriceDB Loading

* [x] Parse pricedb file
* [ ] Serialize pricedb file
* [ ] Fetch new prices and store

### Reports

* [ ] Balance report
* [ ] Register report
* [ ] Net worth report


[ledger]: http://www.ledger-cli.org/
[penny]: http://massysett.github.io/penny/