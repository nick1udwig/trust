#[trust::module]
mod verified {
    trust::total! {
        given ghost {
            sorted(xs);
        }

        pub fn first_sorted(xs: &[i32]) -> i32 {
            xs[0]
        }
    }
}
