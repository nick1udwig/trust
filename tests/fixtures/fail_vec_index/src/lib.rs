#[trust::module]
mod verified {
    trust::total! {
        pub fn get_vec(xs: Vec<i32>, i: usize) -> i32 {
            xs[i]
        }
    }
}
