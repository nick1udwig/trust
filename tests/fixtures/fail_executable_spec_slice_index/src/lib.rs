#[trust::module]
mod verified {
    trust::spec! {
        executable fn first_positive(xs: &[i32]) -> bool {
            xs[0] > 0
        }
    }

    trust::total! {
        pub fn calls_spec(xs: &[i32]) -> bool {
            first_positive(xs)
        }
    }
}
