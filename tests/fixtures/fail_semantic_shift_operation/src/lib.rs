#[trust::module]
mod verified {
    trust::total! {
        pub fn shift_left(x: u32, n: u32) -> u32 {
            x << n
        }
    }
}
