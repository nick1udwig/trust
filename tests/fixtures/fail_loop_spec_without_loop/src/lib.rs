#[trust::module]
mod verified {
    trust::total! {
        pub fn id(n: usize) -> usize {
            trust::loop_spec! {
                decreases(n);
            }
            n
        }
    }
}
