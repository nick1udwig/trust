use trust::TrustModel;

#[derive(TrustModel)]
pub struct Account {
    pub balance: i64,
}

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        given executable {
            acct.balance < i64::MAX;
        }

        pub fn reward_alias(acct: Account) -> i64 {
            let alias = acct;
            alias.balance + 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn reward_alias_increments_balance() {
        assert_eq!(super::verified::reward_alias(Account { balance: 41 }), 42);
    }
}
