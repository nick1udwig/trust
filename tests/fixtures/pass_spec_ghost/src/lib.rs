trust::spec! {
    ghost fn sorted(xs: &[i32]) -> bool {
        forall(|i: usize, j: usize| i < j && j < xs.len())
    }
}

pub fn ordinary_rust_still_builds() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn ghost_spec_is_erased() {
        assert_eq!(super::ordinary_rust_still_builds(), 1);
    }
}
