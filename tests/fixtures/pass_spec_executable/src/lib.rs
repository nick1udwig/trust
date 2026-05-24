trust::spec! {
    executable fn nonempty(xs: &[i32]) -> bool {
        xs.len() > 0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn executable_spec_is_runtime_rust() {
        assert!(super::nonempty(&[1]));
        assert!(!super::nonempty(&[]));
    }
}
