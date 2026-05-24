#[trust::module]
mod verified {
    trust::total! {
        pub fn countdown(mut n: usize) -> usize {
            trust::loop_spec! {
                decreases(n);
            }
            trust::loop_spec! {
                decreases(n);
            }
            while n > 0 {
                n = n - 1;
            }
            n
        }
    }
}
