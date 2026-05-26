#[trust::module]
mod verified {
    trust::total! {
        pub fn add_one_u32(x: u32) -> u32 { x + 1 }
    }
}
