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

        pub fn reward(acct: Account) -> i64 {
            acct.balance + 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn reward_increments_balance() {
        assert_eq!(super::verified::reward(Account { balance: 41 }), 42);
    }
}

