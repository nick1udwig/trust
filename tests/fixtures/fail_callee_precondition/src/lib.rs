#[trust::module]
mod verified {
    trust::total! {
        given executable {
            i < xs.len();
        }

        pub fn get(xs: &[i32], i: usize) -> i32 {
            xs[i]
        }
    }

    trust::total! {
        pub fn bad_first(xs: &[i32]) -> i32 {
            get(xs, 0)
        }
    }
}
