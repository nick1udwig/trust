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
        }

        pub fn withdraw(acct: Account, amount: i64) -> Account {
            Account { id: 0, balance: acct.balance - amount }
        }
    }
}
