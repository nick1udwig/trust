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
            amount >= 0;
        }

        pub fn withdraw_if_safe(acct: Account, amount: i64) -> i64 {
            if acct.balance >= amount {
                acct.balance - amount
            } else {
                acct.balance
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn withdraw_if_safe_only_subtracts_when_possible() {
        assert_eq!(
            super::verified::withdraw_if_safe(Account { balance: 100 }, 30),
            70
        );
        assert_eq!(
            super::verified::withdraw_if_safe(Account { balance: 20 }, 30),
            20
        );
    }
}

