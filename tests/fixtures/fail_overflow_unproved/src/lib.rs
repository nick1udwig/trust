#[trust::module]
mod verified {
    trust::total! {
        pub fn add_one(x: i32) -> i32 {
            x + 1
        }
    }
}
