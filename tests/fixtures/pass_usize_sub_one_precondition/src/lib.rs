#[trust::module]
mod verified {
    trust::total! {
        given executable {
            n >= 1;
        }

        pub fn pred(n: usize) -> usize {
            n - 1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pred_works_inside_precondition() {
        assert_eq!(super::verified::pred(9), 8);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in pred: n >= 1")]
    fn pred_rejects_zero() {
        super::verified::pred(0);
    }
}
