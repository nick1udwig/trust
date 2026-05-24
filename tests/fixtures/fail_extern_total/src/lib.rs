#[trust::module]
mod verified {
    trust::total! {
        pub extern "C" fn id_i32(x: i32) -> i32 { x }
    }
}
