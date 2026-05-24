#[trust::module]
mod verified {
    trust::total! {
        pub fn rem(x: i32, y: i32) -> i32 {
            x % y
        }
    }
}
