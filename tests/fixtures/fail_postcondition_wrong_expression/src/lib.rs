#[trust::module]
mod verified {
    trust::total! {
        gives executable |out| {
            out == 1;
        }

        pub fn id_i32(x: i32) -> i32 {
            x
        }
    }
}
