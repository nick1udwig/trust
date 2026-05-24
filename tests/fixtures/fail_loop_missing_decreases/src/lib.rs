#[trust::module]
mod verified {
    trust::total! {
        pub fn count_up(mut i: usize, n: usize) -> usize {
            trust::loop_spec! {
                invariant(i <= n);
            }
            while i < n {
                i += 1;
            }
            i
        }
    }
}
