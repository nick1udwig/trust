#[trust::module]
mod verified {
    trust::total! {
        pub fn id_i32(x: i32) -> i32 { x }
    }
}
