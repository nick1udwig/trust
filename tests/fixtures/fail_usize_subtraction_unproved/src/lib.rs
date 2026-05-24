#[trust::module]
mod verified {
    trust::total! {
        pub fn pred(n: usize) -> usize {
            n - 1
        }
    }
}
