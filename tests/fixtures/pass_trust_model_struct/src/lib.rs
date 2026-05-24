use trust::TrustModel;

#[derive(TrustModel)]
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

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn reads_modeled_field() {
        assert_eq!(
            super::verified::balance(Account {
                id: 7,
                balance: 42
            }),
            42
        );
    }
}
