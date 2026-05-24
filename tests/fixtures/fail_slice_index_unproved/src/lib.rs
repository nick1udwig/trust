#[trust::module]
mod verified {
    trust::total! {
        pub fn first(xs: &[i32]) -> i32 {
            xs[0]
        }
    }
}
