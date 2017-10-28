use std::rc::Rc;
use super::amount::Amount;
use super::header::Header;

#[derive(PartialEq, Debug)]
pub enum AmountSource {
    Provided,
    Inferred,
}

#[derive(PartialEq, Debug)]
pub struct Posting {
    header: Rc<Header>,
    account: String,
    account_lineage: Vec<String>, 
    amount: Amount,
    amount_source: AmountSource,
    comment: Option<String>,
}

impl Posting {
    pub fn new(header: Rc<Header>, account: String,
    sub_accounts: &Vec<String>, amount: Amount, amount_source: AmountSource,
    comment: Option<String>) -> Posting {
        let account_lineage = build_account_lineage(sub_accounts);
        Posting {
            header,
            account,
            account_lineage,
            amount,
            amount_source,
            comment
        }
    }
}


/// Build a vector of full account names for all levels of accounts based on the
/// `sub_accounts` provided.
///
/// e.g. Given ["Assets", "Savings", "Bank"] we should get back ["Assets",
/// "Assets:Savings", "Assets:Savings:Bank"]
fn build_account_lineage(sub_accounts: &Vec<String>) -> Vec<String> {
    let mut account_lineage = Vec::new();
    let mut account = String::new();

    for sub_account in sub_accounts.iter() {
        if account.len() == 0 {
            account.push_str(sub_account);
        } else {
            account.push(':');
            account.push_str(sub_account);
        }
        account_lineage.push(account.clone());
    }

    account_lineage
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_account_lineage_should_provide_full_account_name_for_all_levels() {
        let sub_accounts = vec!["Assets".to_string(), "Savings".to_string(), "Bank".to_string()];
        let expected = vec!["Assets", "Assets:Savings", "Assets:Savings:Bank"];
        assert_eq!(build_account_lineage(&sub_accounts), expected);
    }
}