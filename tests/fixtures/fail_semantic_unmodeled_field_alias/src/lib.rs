pub struct Account {
    pub balance: i64,
}

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        pub fn balance_alias(acct: Account) -> i64 {
            let alias = acct;
            alias.balance
        }
    }
}
