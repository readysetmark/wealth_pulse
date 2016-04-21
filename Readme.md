Wealth Pulse
============

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


Project Plan
------------

PriceDB
* [x] Parse pricedb file
* [ ] Serialize pricedb file
* [ ] Fetch new prices and store

Ledger
* [ ] Parse ledger file

Reports
* [ ] Balance report
* [ ] Register report
* [ ] Net worth report


[ledger]: http://www.ledger-cli.org/
[penny]: http://massysett.github.io/penny/