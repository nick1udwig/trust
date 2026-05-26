#[trust::module]
mod verified {
    trust::total! {
        pub fn invert(x: u32) -> u32 {
            !x
        }
    }
}
