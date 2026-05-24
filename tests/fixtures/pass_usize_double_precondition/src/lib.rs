#[trust::module]
mod verified {
    trust::total! {
        given executable {
            n <= usize::MAX / 2;
        }

        pub fn double(n: usize) -> usize {
            n * 2
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn double_works_inside_precondition() {
        assert_eq!(super::verified::double(21), 42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in double: n <= usize::MAX / 2")]
    fn double_rejects_upper_boundary_input() {
        super::verified::double(usize::MAX / 2 + 1);
    }
}
