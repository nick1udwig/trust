#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 1;
        }

        pub fn zero() -> i32 {
            0
        }
    }
}
