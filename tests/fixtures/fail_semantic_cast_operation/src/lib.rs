#[trust::module]
mod verified {
    trust::total! {
        pub fn narrow(x: u64) -> u32 {
            x as u32
        }
    }
}
