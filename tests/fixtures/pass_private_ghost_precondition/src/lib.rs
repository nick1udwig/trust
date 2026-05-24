#[trust::module]
mod verified {
    trust::total! {
        given ghost {
            sorted(xs);
        }

        given executable {
            xs.len() > 0;
        }

        fn private_first_sorted(xs: &[i32]) -> i32 {
            xs[0]
        }
    }
}
