use trust::TrustModel;

#[derive(TrustModel)]
pub struct Account {
    pub balance: i64,
}

#[trust::module]
mod verified {
    use super::Account;

    trust::total! {
        pub fn reward(acct: Account) -> i64 {
            acct.balance + 1
        }
    }
}

