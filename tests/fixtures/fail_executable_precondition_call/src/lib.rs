#[trust::module]
mod verified {
    trust::total! {
        given executable {
            valid_index(i);
        }

        pub fn id_i32(i: i32) -> i32 { i }
    }
}
