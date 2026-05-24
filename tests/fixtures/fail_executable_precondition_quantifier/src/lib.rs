#[trust::module]
mod verified {
    trust::total! {
        given executable {
            forall i;
        }

        pub fn id_i32(x: i32) -> i32 { x }
    }
}
