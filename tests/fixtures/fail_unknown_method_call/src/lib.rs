#[trust::module]
mod verified {
    trust::total! {
        pub fn abs_value(x: i32) -> i32 {
            x.abs()
        }
    }
}
