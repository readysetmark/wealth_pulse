use decimal::d128;
use std::fmt;
use super::symbol::Symbol;


#[derive(PartialEq, Debug, Clone)]
pub enum SymbolPosition {
    Left,
    Right,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Spacing {
    Space,
    NoSpace,
}

#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
pub struct Amount {
    pub quantity: d128,
    pub symbol: Symbol,
    pub render_options: RenderOptions,
}

impl Amount {
    pub fn new(quantity: d128, symbol: Symbol, render_opts: RenderOptions) -> Amount {
        Amount {
            quantity: quantity,
            symbol: symbol,
            render_options: render_opts,
        }
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let spacing =
            match self.render_options.spacing {
                Spacing::Space => " ",
                Spacing::NoSpace => "",
            };

        match self.render_options.symbol_position {
            SymbolPosition::Left => write!(f, "{}{}{}", self.symbol, spacing, self.quantity),
            SymbolPosition::Right => write!(f, "{}{}{}", self.quantity, spacing, self.symbol),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use core::symbol::*;

    #[test]
    fn amount_fmt_symbol_left_with_space() {
        let result = format!("{}", Amount::new(
            d128!(13245.00),
            Symbol::new("US$", QuoteOption::Unquoted),
            RenderOptions::new(SymbolPosition::Left, Spacing::Space)));
        assert_eq!(result, "US$ 13245.00");
    }

    #[test]
    fn amount_fmt_symbol_left_no_space() {
        let result = format!("{}", Amount::new(
                d128!(13245.00),
                Symbol::new("$", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Left, Spacing::NoSpace)));
        assert_eq!(result, "$13245.00");
    }

    #[test]
    fn amount_fmt_symbol_right_with_space() {
        let result = format!("{}", Amount::new(
                d128!(13245.463),
                Symbol::new("MUTF2351", QuoteOption::Quoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::Space)));
        assert_eq!(result, "13245.463 \"MUTF2351\"");
    }

    #[test]
    fn amount_fmt_symbol_right_no_space() {
        let result = format!("{}", Amount::new(
                d128!(13245.463),
                Symbol::new("RUST", QuoteOption::Unquoted),
                RenderOptions::new(SymbolPosition::Right, Spacing::NoSpace)));
        assert_eq!(result, "13245.463RUST");
    }
}