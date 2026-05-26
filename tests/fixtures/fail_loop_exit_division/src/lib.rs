#[trust::module]
mod verified {
    trust::total! {
        pub fn divide_after_countdown(mut n: usize) -> usize {
            trust::loop_spec! {
                decreases(n);
            }
            while n > 0 {
                n = n - 1;
            }
            1 / n
        }
    }
}
