#[trust::module]
mod verified {
    trust::total! {
        pub fn bad_unwrap(x: Option<i32>) -> i32 {
            x.unwrap()
        }
    }
}
