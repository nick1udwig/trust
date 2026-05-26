#[trust::module]
mod verified {
    trust::total! {
        pub fn divide_after_decrement(mut n: usize) -> usize {
            let mut out: usize = 0;
            trust::loop_spec! {
                decreases(n);
            }
            while n > 0 {
                n = n - 1;
                out = 1 / n;
            }
            out
        }
    }
}
