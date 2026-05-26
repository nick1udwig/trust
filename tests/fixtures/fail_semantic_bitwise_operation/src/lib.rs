#[trust::module]
mod verified {
    trust::total! {
        pub fn mask(x: u32, bits: u32) -> u32 {
            x & bits
        }
    }
}
