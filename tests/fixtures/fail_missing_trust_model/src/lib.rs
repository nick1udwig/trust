pub struct Account {
    pub id: u64,
    pub balance: i64,
}

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        pub fn balance(acct: Account) -> i64 {
            acct.balance
        }
    }
}
