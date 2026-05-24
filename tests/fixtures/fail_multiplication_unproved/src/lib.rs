#[trust::module]
mod verified {
    trust::total! {
        pub fn double(x: i32) -> i32 {
            x * 2
        }
    }
}
