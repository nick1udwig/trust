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
        given executable {
            amount >= 0;
            acct.balance >= amount;
        }

        gives ghost |out| {
            out.id == old(acct.id);
            int(out.balance) == int(old(acct.balance)) - int(old(amount));
        }

        pub fn withdraw(acct: Account, amount: i64) -> Account {
            Account { id: acct.id, balance: acct.balance - amount }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Account;

    #[test]
    fn withdraw_preserves_id_and_reduces_balance() {
        let out = super::verified::withdraw(Account { id: 7, balance: 100 }, 30);

        assert_eq!(out.id, 7);
        assert_eq!(out.balance, 70);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in withdraw: acct.balance >= amount")]
    fn withdraw_rejects_overdraft_at_boundary() {
        let _ = super::verified::withdraw(Account { id: 7, balance: 20 }, 30);
    }
}
