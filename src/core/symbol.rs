use std::fmt;


#[derive(PartialEq, Debug)]
pub enum QuoteOption {
    Quoted,
    Unquoted
}

#[derive(PartialEq, Debug)]
pub struct Symbol {
    value: String,
    quote_option: QuoteOption
}

impl Symbol {
    pub fn new<S>(symbol: S, quote_option: QuoteOption) -> Symbol
    where S: Into<String> {
        Symbol {
            value: symbol.into(),
            quote_option: quote_option
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.quote_option {
            QuoteOption::Quoted   => write!(f, "\"{}\"", self.value),
            QuoteOption::Unquoted => write!(f, "{}", self.value),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_fmt_quoted() {
        let result =
            format!("{}", Symbol::new("MUTF2351", QuoteOption::Quoted));
        assert_eq!(result, "\"MUTF2351\"");
    }

    #[test]
    fn symbol_fmt_unquoted() {
        let result =
            format!("{}", Symbol::new("$", QuoteOption::Unquoted));
        assert_eq!(result, "$");
    }
}