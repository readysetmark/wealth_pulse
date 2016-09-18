use decimal::d128;
use std::fmt;
use super::symbol::Symbol;


#[derive(PartialEq, Debug)]
pub enum SymbolPosition {
    Left,
    Right,
}

#[derive(PartialEq, Debug)]
pub enum Spacing {
    Space,
    NoSpace,
}

#[derive(PartialEq, Debug)]
pub struct RenderOptions {
    symbol_position: SymbolPosition,
    spacing: Spacing,
}

impl RenderOptions {
    pub fn new(position: SymbolPosition, spacing: Spacing) -> RenderOptions {
        RenderOptions {
            symbol_position: position,
            spacing: spacing,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Instrument {
    amount: d128,
    symbol: Symbol,
    render_options: RenderOptions,
}

impl Instrument {
    pub fn new(amount: d128, symbol: Symbol, render_opts: RenderOptions) -> Instrument {
        Instrument {
            amount: amount,
            symbol: symbol,
            render_options: render_opts,
        }
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let spacing =
            match self.render_options.spacing {
                Spacing::Space => " ",
                Spacing::NoSpace => "",
            };

        match self.render_options.symbol_position {
            SymbolPosition::Left => write!(f, "{}{}{}", self.symbol, spacing, self.amount),
            SymbolPosition::Right => write!(f, "{}{}{}", self.amount, spacing, self.symbol),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::symbol::*;

    #[test]
    fn instrument_fmt_symbol_left_with_space() {
        let result = format!("{}", Instrument::new(
            d128!(13245.00),
            Symbol::new("US$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::Space)));
        assert_eq!(result, "US$ 13245.00");
    }

    #[test]
    fn instrument_fmt_symbol_left_no_space() {
        let result = format!("{}", Instrument::new(
                d128!(13245.00),
                Symbol::new("$", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)));
        assert_eq!(result, "$13245.00");
    }

    #[test]
    fn instrument_fmt_symbol_right_with_space() {
        let result = format!("{}", Instrument::new(
                d128!(13245.463),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::Space)));
        assert_eq!(result, "13245.463 \"MUTF2351\"");
    }

    #[test]
    fn instrument_fmt_symbol_right_no_space() {
        let result = format!("{}", Instrument::new(
                d128!(13245.463),
                Symbol::new("RUST", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::NoSpace)));
        assert_eq!(result, "13245.463RUST");
    }
}