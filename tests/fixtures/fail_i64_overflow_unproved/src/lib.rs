#[trust::module]
mod verified {
    trust::total! {
        pub fn add_one_i64(x: i64) -> i64 { x + 1 }
    }
}
