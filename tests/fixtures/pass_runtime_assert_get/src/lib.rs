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
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_returns_in_bounds_value() {
        assert_eq!(super::verified::get(&[4, 8, 15], 1), 8);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in get: i < xs.len()")]
    fn get_bad_index_panics_with_trust_precondition() {
        super::verified::get(&[], 0);
    }
}
