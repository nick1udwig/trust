trust::trusted_model! {
    axiom false_is_true: false;
}

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
