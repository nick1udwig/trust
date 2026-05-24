trust::spec! {
    executable fn bad(xs: &[i32]) -> bool {
        forall(|i: usize| i < xs.len())
    }
}
