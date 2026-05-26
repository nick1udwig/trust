#[trust::module]
mod verified {
    trust::total! {
        pub fn rem_neg_one(x: i32) -> i32 {
            x % -1
        }
    }
}
