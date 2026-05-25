#[trust::module]
mod verified {
    trust::total! {
        pub fn divide(x: i32, y: i32) -> i32 {
            x / { y }
        }
    }
}
