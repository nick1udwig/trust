#[trust::module]
mod verified {
    trust::total! {
        pub fn countdown(mut n: usize) -> usize {
            while n > 0 {
                n = n - 1;
            }
            n
        }
    }
}
