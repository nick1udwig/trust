#[trust::module]
mod verified {
    trust::total! {
        pub fn count_up(mut i: usize, n: usize) -> usize {
            trust::loop_spec! {
                invariant(i <= n);
                decreases(n - i);
            }
            while i < n {
                i = i + 2;
            }
            i
        }
    }
}
