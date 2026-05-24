#[trust::module]
mod verified {
    trust::total! {
        given executable {
            i < xs.len();
        }

        pub fn get(xs: &[i32], i: usize) -> i32 {
            xs[i]
        }
    }

    trust::total! {
        given executable {
            xs.len() > 0;
        }

        pub fn first(xs: &[i32]) -> i32 {
            get(xs, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn first_uses_verified_get() {
        assert_eq!(super::verified::first(&[42]), 42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in first: xs.len() > 0")]
    fn first_empty_slice_panics_with_trust_precondition() {
        super::verified::first(&[]);
    }
}
