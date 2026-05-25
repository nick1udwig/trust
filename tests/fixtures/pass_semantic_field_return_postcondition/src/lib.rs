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
        gives ghost |out| {
            out == acct.balance;
        }

        pub fn balance(acct: Account) -> i64 {
            let out = acct.balance;
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn field_return_postcondition_uses_mir_projection() {
        assert_eq!(
            super::verified::balance(Account {
                id: 7,
                balance: 42
            }),
            42
        );
    }
}
