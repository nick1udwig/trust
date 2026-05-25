use trust::TrustModel;

#[derive(TrustModel)]
pub struct Bag {
    pub values: Vec<i32>,
}
